use crate::compute::*;
use crate::compute_graph::*;
use std::collections::{HashMap, HashSet};
use std::any::{Any, TypeId, type_name};
use slotmap::{SlotMap, new_key_type};
new_key_type!{struct GraphKey;}

#[derive(Clone)]
pub(crate) struct Node<'a> {
    name: String,
    inputs: Vec<GraphKey>,    
    inner: Box<dyn InnerCompute + 'a>
}

pub struct NodeHandle {
    key: GraphKey,
    graph_id: usize,
}

#[derive(Clone)]
pub struct Graph<'a> {
    type_names: HashMap<TypeId, &'a str>,
    nodes: SlotMap<GraphKey, Node<'a>>,
    output_node: Option<GraphKey>,
    id: usize,    
}


impl<'a> Graph<'a> {
    pub fn new() -> Self {
        let mut g = Self {
            type_names: HashMap::default(),
            nodes: SlotMap::default(),
            output_node: None,
            id: 0,            
        };

        g.id = (&g.nodes as *const SlotMap<_,_>) as usize;
        g        
    }

    pub fn insert_node<N, In, Out>(&mut self, name: N, compute_func: impl Compute<In = In, Out = Out> + 'static) -> NodeHandle   
    where  
        N: Into<String>, 
        In: Any + Copy + Default + 'static,
        Out: Any + Copy + Default + 'static
    {
        let node = Node {
            name: name.into(),
            inputs: Vec::new(),
            inner: Box::new(compute_func),
        };

        self.type_names.insert(TypeId::of::<In>(), type_name::<In>());
        self.type_names.insert(TypeId::of::<Out>(), type_name::<Out>());
        
        let key = self.nodes.insert(node);
        NodeHandle {
            key,
            graph_id: self.id,
        }
    }

    pub fn add_input(&mut self, node_handle: &NodeHandle, input_node_handle: &NodeHandle) -> Result<(), ComputeGraphErrors>{
        let node_input_type = self.nodes.get(node_handle.key).unwrap().inner.input_type();
        let input_node_output_type = self.nodes.get(input_node_handle.key).unwrap().inner.output_type();
        if node_input_type == input_node_output_type {
            let node = self.nodes.get_mut(node_handle.key).unwrap();
            node.inputs.push(input_node_handle.key);            
            Ok(())
        } else {
            Err(ComputeGraphErrors::WrongTypes(
                format!("Node Input {}: {}", self._get_name(node_handle.key).unwrap(), self.type_names.get(&node_input_type).unwrap()),
                format!("Input Node Output {}: {}", self._get_name(input_node_handle.key).unwrap(), self.type_names.get(&input_node_output_type).unwrap()))
            )
        }        
    }

    pub fn get_name(&self, node_handle: &NodeHandle) -> Result<String, ComputeGraphErrors> {
        self._get_name(node_handle.key)        
    }

    fn _get_name(&self, node_key: GraphKey) -> Result<String, ComputeGraphErrors> {
        let node = self.nodes.get(node_key).ok_or(ComputeGraphErrors::NodeMissing)?;
        Ok(node.name.clone())
    }

    pub fn set_output_node(&mut self, node_handle: &NodeHandle) {
        self.output_node = Some(node_handle.key);
    }    

    pub fn build<In, Out>(&self) -> Result<ComputeGraph<In, Out>, ComputeGraphErrors> 
    where 
        In: Any + Copy,
        Out: Any + Copy
    {
        let output_node = self.output_node.ok_or(ComputeGraphErrors::NoOutputNode)?;
        let compute_order = self.compute_order(output_node)?;        

        let node_key_to_index = compute_order.iter().enumerate().map(|(i, key)| (*key, i)).collect::<HashMap<_, _>>();
        
        let nodes = compute_order.iter().map(|key| {
            let node = self.nodes.get(*key).unwrap();            
            let inputs = node.inputs.iter().map(|input_key| *node_key_to_index.get(input_key).unwrap()).collect::<Vec<_>>();                        
            
            ComputeNode {
                inputs,
                func: node.inner.clone()
            }

        }).collect::<Vec<_>>();

        Ok(ComputeGraph::new(nodes))          
    }

    fn compute_order(&self, node: GraphKey) -> Result<Vec<GraphKey>, ComputeGraphErrors> {
        let mut compute_order = Vec::new();
        let mut temp_list = HashSet::new();
        self.toposort_visit(node, &mut compute_order, &mut temp_list)?;
        Ok(compute_order)
    }

    /// Adapted from the DFS-based toposort of https://en.wikipedia.org/wiki/Topological_sorting
    fn toposort_visit(&self, node: GraphKey, sorted_list: &mut Vec<GraphKey>, temp_list: &mut HashSet<GraphKey>) -> Result<(), ComputeGraphErrors>{        
        if sorted_list.contains(&node) {
            return Ok(());
        }

        if temp_list.contains(&node) {
            return Err(ComputeGraphErrors::GraphCycle(self._get_name(node).unwrap()));
        }

        temp_list.insert(node);
        
        for input_node in self.nodes.get(node).unwrap().inputs.iter() {
            self.toposort_visit(*input_node, sorted_list, temp_list)?;
        }

        temp_list.remove(&node);
        sorted_list.push(node);
        Ok(())
    }
}

#[derive(Debug)]
pub enum ComputeGraphErrors {
    GraphCycle(String),
    NoOutputNode,
    NodeMissing,
    WrongTypes(String, String)
}

#[cfg(test)]
mod graph_tests {
    use crate::{graph::*, compute_graph};
    #[test]
    fn test_closure_node() -> Result<(), ComputeGraphErrors> {
        let mut graph = Graph::new();
        let multi = |input: &[&f64]| -> f64 {
            42.0 * input[0]
        };
        let multi_handle = graph.insert_node("multiplier", multi as fn(&[&f64]) -> f64);

        graph.set_output_node(&multi_handle);
        let mut graph2 = graph.clone();
        let compute_graph = graph.build::<f64, f64>()?;        

        let summer = |input: &[&f64]| -> f64 {
            input.iter().map(|v| *v).sum()
        };

        let sum_handle = graph2.insert_node("summer", summer as fn(&[&f64]) -> f64);
        graph2.add_input(&sum_handle, &multi_handle)?;

        let constant = |_:&[&()]| -> f64 {
            11.0
        };
        let const_handle = graph2.insert_node("constant", constant as fn(&[&()]) -> f64);
        graph2.add_input(&sum_handle, &const_handle)?;
        graph2.set_output_node(&sum_handle);
        
        let compute_graph2 = graph2.build::<f64, f64>()?;
        
        eprintln!("cg1 {}", compute_graph.compute(&7.0));        
        eprintln!("cg2 {}", compute_graph2.compute(&7.0));

        Ok(())
    }
}
