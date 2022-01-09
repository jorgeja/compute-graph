use crate::com_graph::*;
use crate::compute::*;
use slotmap::{new_key_type, SlotMap};
use std::any::{type_name, Any, TypeId};
use std::collections::{HashMap, HashSet};
new_key_type! {struct GraphKey;}

#[derive(Clone)]
struct Node {
    name: String,
    inputs: Vec<GraphKey>,
    inner: Box<dyn InnerCompute + 'static>,
    connected_to_input: bool,
}

#[derive(Clone, Copy)]
pub struct NodeHandle {
    key: GraphKey,
    graph_id: usize,
}

pub struct NodeMeta {
    pub this_node: NodeHandle,
    pub inputs: Vec<NodeHandle>,
    pub connected_to_input: bool,
    pub input_type: TypeId,
    pub output_type: TypeId,
}

#[derive(Clone)]
pub struct Graph {
    type_names: HashMap<TypeId, &'static str>,
    nodes: SlotMap<GraphKey, Node>,
    output_node: Option<GraphKey>,
    id: usize,
}

impl Default for Graph {
    fn default() -> Self {
        Graph::new()
    }
}

impl Graph {
    pub fn new() -> Self {
        let mut g = Self {
            type_names: HashMap::default(),
            nodes: SlotMap::default(),
            output_node: None,
            id: 0,
        };

        g.id = (&g.nodes as *const SlotMap<_, _>) as usize;
        g
    }

    pub fn insert_node<N, Obj, In, Out>(&mut self, name: N, compute_object: Obj) -> NodeHandle
    where
        N: Into<String>,
        Obj: Compute<In = In, Out = Out> + 'static,
        In: Any + Copy + Default + 'static,
        Out: Any + Copy + Default + 'static,
    {
        let node = Node {
            name: name.into(),
            inputs: Vec::new(),
            inner: Box::new(compute_object),
            connected_to_input: true,
        };

        self.type_names
            .insert(TypeId::of::<In>(), type_name::<In>());
        self.type_names
            .insert(TypeId::of::<Out>(), type_name::<Out>());

        let key = self.nodes.insert(node);
        NodeHandle {
            key,
            graph_id: self.id,
        }
    }

    pub fn remove_node(&mut self, node_handle: &NodeHandle) {
        self.verify_graphid(node_handle);
        self.nodes.remove(node_handle.key);
        for (_, node) in self.nodes.iter_mut() {
            node.inputs.retain(|key| *key != node_handle.key);
        }
    }

    pub fn replace_node<Obj, In, Out>(
        &mut self,
        node_handle: &NodeHandle,
        compute_object: Obj,
    ) -> Result<(), ComputeGraphErrors>
    where
        Obj: Compute<In = In, Out = Out> + 'static,
        In: Any + Copy + Default + 'static,
        Out: Any + Copy + Default + 'static,
    {
        self.verify_graphid(node_handle);
        let node = self
            .nodes
            .get_mut(node_handle.key)
            .ok_or(ComputeGraphErrors::NodeMissing)?;

        let new_inner_compute: Box<dyn InnerCompute> = Box::new(compute_object);
        let mut type_errors = Vec::new();
        if new_inner_compute.input_type() != node.inner.input_type() {
            type_errors.push((
                "input",
                *self.type_names.get(&node.inner.input_type()).unwrap(),
                *self
                    .type_names
                    .get(&new_inner_compute.input_type())
                    .unwrap_or(&"unknown type"),
            ))
        }
        if new_inner_compute.output_type() != node.inner.output_type() {
            type_errors.push((
                "output",
                *self.type_names.get(&node.inner.output_type()).unwrap(),
                *self
                    .type_names
                    .get(&new_inner_compute.output_type())
                    .unwrap_or(&"unknown type"),
            ))
        }
        if !type_errors.is_empty() {
            return Err(ComputeGraphErrors::format_incompatible_object(
                &node.name,
                &type_errors,
            ));
        }

        node.inner = new_inner_compute;
        Ok(())
    }
    
    pub fn get_node_meta(&self, node_handle: &NodeHandle) -> NodeMeta {
        self.verify_graphid(node_handle);
        let node = self.nodes.get(node_handle.key).unwrap();
        self.build_node_meta(node_handle.key, node)
    }

    pub fn get_all_node_metas(&self) -> Vec<NodeMeta> {
        self.nodes.iter().map(|(key, node)| self.build_node_meta(key, node)).collect()
    }

    fn build_node_meta(&self, key: GraphKey, node: &Node) -> NodeMeta {
        NodeMeta {
            this_node: NodeHandle {key, graph_id: self.id },
            inputs: node.inputs.iter().map(|key| NodeHandle {key: *key, graph_id: self.id }).collect(),
            connected_to_input: node.connected_to_input,
            input_type: node.inner.input_type(),
            output_type: node.inner.output_type()
        }
    }

    pub fn add_input(
        &mut self,
        node_handle: &NodeHandle,
        input_node_handle: &NodeHandle,
    ) -> Result<(), ComputeGraphErrors> {
        self.verify_graphid(node_handle);
        self.verify_graphid(input_node_handle);
        let node_input_type = &self.nodes[node_handle.key].inner.input_type();
        let input_node_output_type = &self.nodes[input_node_handle.key].inner.output_type();
        if *node_input_type == *input_node_output_type {
            let node = self.nodes.get_mut(node_handle.key).unwrap();
            node.inputs.push(input_node_handle.key);

            if node.connected_to_input {
                node.connected_to_input = false;
            }

            Ok(())
        } else {
            Err(ComputeGraphErrors::format_wrong_types(
                self._get_name(node_handle.key).unwrap(),
                self.type_names.get(node_input_type).unwrap(),
                self._get_name(input_node_handle.key).unwrap(),
                self.type_names.get(input_node_output_type).unwrap(),
            ))
        }
    }

    pub fn remove_input(&mut self, node_handle: &NodeHandle, input_to_remove_handle: &NodeHandle) {
        self.verify_graphid(node_handle);
        if let Some(node) = self.nodes.get_mut(node_handle.key) {
            node.inputs.retain(|key| *key != input_to_remove_handle.key);
        }
    }

    pub fn get_name(&self, node_handle: &NodeHandle) -> Result<String, ComputeGraphErrors> {
        self.verify_graphid(node_handle);
        let name = self._get_name(node_handle.key)?;
        Ok(name.to_string())
    }

    pub fn get_type_name(&self, type_id: TypeId) -> Option<&'static str> {
        self.type_names.get(&type_id).map(|v| *v)
    }

    pub fn set_output_node(&mut self, node_handle: &NodeHandle) {
        self.verify_graphid(node_handle);
        self.output_node = Some(node_handle.key);
    }

    pub fn connect_to_input(&mut self, node_handle: &NodeHandle) {
        self.verify_graphid(node_handle);
        if let Some(node) = self.nodes.get_mut(node_handle.key) {
            node.connected_to_input = true;
        }
    }

    pub fn disconnect_from_input(&mut self, node_handle: &NodeHandle) {
        self.verify_graphid(node_handle);
        if let Some(node) = self.nodes.get_mut(node_handle.key) {
            node.connected_to_input = false;
        }
    }

    pub fn build<In, Out>(&mut self) -> Result<ComputeGraph<In, Out>, ComputeGraphErrors>
    where
        In: Any + Copy,
        Out: Any + Copy,
    {
        let output_node_key = self.output_node.ok_or(ComputeGraphErrors::NoOutputNode)?;
        self._build_for_node(output_node_key)
    }

    pub fn build_for_node<In, Out>(
        &mut self,
        output_node_handle: &NodeHandle,
    ) -> Result<ComputeGraph<In, Out>, ComputeGraphErrors>
    where
        In: Any + Copy,
        Out: Any + Copy,
    {
        self.verify_graphid(output_node_handle);
        self._build_for_node(output_node_handle.key)
    }

    fn _build_for_node<In, Out>(
        &mut self,
        output_node_key: GraphKey,
    ) -> Result<ComputeGraph<In, Out>, ComputeGraphErrors>
    where
        In: Any + Copy,
        Out: Any + Copy,
    {
        let output_node_output_typeid = self.nodes[output_node_key].inner.output_type();
        let output_typeid = TypeId::of::<Out>();
        if output_node_output_typeid != output_typeid {
            return Err(ComputeGraphErrors::format_wrong_types(
                "compute output",
                self.type_names
                    .get(&output_typeid)
                    .unwrap_or(&"unknown type"),
                self._get_name(output_node_key).unwrap(),
                self.type_names.get(&output_node_output_typeid).unwrap(),
            ));
        }

        let compute_order = self.compute_order(output_node_key)?;
        let input_typeid = TypeId::of::<In>();

        let node_key_to_index = compute_order
            .iter()
            .enumerate()
            .map(|(i, key)| (*key, i))
            .collect::<HashMap<_, _>>();

        let mut nodes = Vec::new();
        let mut num_connected_to_input = 0;
        for node_key in compute_order {
            let node = &self.nodes[node_key];
            if node.connected_to_input {
                num_connected_to_input += 1;
                if node.inner.input_type() != TypeId::of::<()>()
                    && node.inner.input_type() != input_typeid
                {
                    return Err(ComputeGraphErrors::format_wrong_types(
                        self._get_name(node_key).unwrap(),
                        self.type_names.get(&node.inner.input_type()).unwrap(),
                        "compute input",
                        self.type_names
                            .get(&input_typeid)
                            .unwrap_or(&"unknown type"),
                    ));
                }
            }

            let inputs = node
                .inputs
                .iter()
                .map(|input_key| *node_key_to_index.get(input_key).unwrap())
                .collect::<Vec<_>>();

            nodes.push(ComputeNode {
                connected_to_input: node.connected_to_input,
                inputs,
                func: node.inner.clone(),
            });
        }

        if num_connected_to_input == 0 {
            return Err(ComputeGraphErrors::NoInputNodes);
        }

        Ok(ComputeGraph::new(nodes))
    }

    fn compute_order(&self, node: GraphKey) -> Result<Vec<GraphKey>, ComputeGraphErrors> {
        let mut compute_order = Vec::new();
        let mut temp_list = HashSet::new();
        self.toposort_visit(node, &mut compute_order, &mut temp_list)?;
        Ok(compute_order)
    }

    fn toposort_visit(
        &self,
        node: GraphKey,
        sorted_list: &mut Vec<GraphKey>,
        temp_list: &mut HashSet<GraphKey>,
    ) -> Result<(), ComputeGraphErrors> {
        if sorted_list.contains(&node) {
            return Ok(());
        }

        if temp_list.contains(&node) {
            return Err(ComputeGraphErrors::GraphCycle(
                self._get_name(node).unwrap().to_string(),
            ));
        }

        temp_list.insert(node);

        for input_node in self.nodes.get(node).unwrap().inputs.iter() {
            self.toposort_visit(*input_node, sorted_list, temp_list)?;
        }

        temp_list.remove(&node);
        sorted_list.push(node);
        Ok(())
    }

    fn _get_name(&self, node_key: GraphKey) -> Result<&str, ComputeGraphErrors> {
        let node = self
            .nodes
            .get(node_key)
            .ok_or(ComputeGraphErrors::NodeMissing)?;
        Ok(&node.name)
    }

    fn verify_graphid(&self, node_handle: &NodeHandle) {
        if node_handle.graph_id != self.id {
            panic!(
                "Graph got node_handle with wrong graph_id: {} != {}",
                node_handle.graph_id, self.id
            );
        }
    }
}


#[derive(Debug)]
pub enum ComputeGraphErrors {
    NoInputNodes,
    NoOutputNode,
    NodeMissing,
    IncompatibleNewNode(String),
    GraphCycle(String),
    WrongTypes(String),
}

impl ComputeGraphErrors {
    fn format_wrong_types(
        input_name: &str,
        input_type: &str,
        output_name: &str,
        output_type: &str,
    ) -> Self {
        Self::WrongTypes(format!(
            "'{}' input type '{}' does not match '{}' output type '{}'",
            input_name, input_type, output_name, output_type
        ))
    }
    fn format_incompatible_object(
        input_name: &str,
        incompatible_types: &[(&str, &str, &str)],
    ) -> Self {
        let mut msg = format!("Can't replace '{}' because: ", input_name);
        for (i, (slot_name, old_type_name, new_type_name)) in incompatible_types.iter().enumerate()
        {
            if i > 0 {
                msg += ", ";
            }
            msg += &format!(
                "'{}'s old type '{}' != new type '{}'",
                slot_name, old_type_name, new_type_name
            );
        }
        Self::IncompatibleNewNode(msg)
    }
}

#[cfg(test)]
mod graph_tests {
    use crate::{
        graph::*,
        operations::{AddInputs, Constant, MulInputs},
    };
    #[test]
    fn test_functionality() -> Result<(), ComputeGraphErrors> {
        //  Building this graph:
        //  Input : f64   Constant : f64
        //       |          |   |
        //       \__ mul __/    |
        //           |          |
        //           \__ add __/
        //                |
        //          Output : f64

        let mut graph = Graph::new();

        //Constant value
        let const_handle = graph.insert_node("the_answer", Constant(42.0));
        //Adds all inputs to this node
        let add_handle = graph.insert_node("add", AddInputs::<f64>::new());
        //Multiplies all the inputs to this node
        let mul_handle = graph.insert_node("mul", MulInputs::<f64>::new());

        //Operation fails if output type does not match the input type
        match graph.add_input(&add_handle, &mul_handle) {
            Err(msg) => eprintln!("{:?}", msg),
            _ => {}
        };

        //Lets setup the rest of the nodes and ignore errors..
        graph.add_input(&add_handle, &const_handle)?;
        graph.add_input(&mul_handle, &const_handle)?;

        //By default the graph sends the input to any input that has no other inputs
        //If you want a node to have the input together with other nodes you have to manually assign it
        graph.connect_to_input(&mul_handle);

        //We can specify an output node:
        graph.set_output_node(&add_handle);
        //We must build a ComputeGraph before we can compute anything
        //Graph fails if input type does not match output type, or there are cycles in the graph.
        let compute_graph = graph.build::<f64, f64>()?;
        let v = compute_graph.compute(&7.0);
        assert_eq!(v, 336.0);

        //If we want to compute just part of the graph we can specify a node:
        let sub_compute_graph = graph.build_for_node::<f64, f64>(&mul_handle)?;
        let v = sub_compute_graph.compute(&7.0);
        assert_eq!(v, 294.0);

        //Lets replace the constant value. Will fail if the input/output types don't match up
        graph.replace_node(&const_handle, Constant(11.0))?;

        //We must rebuild the graph after manipulating inputs/edges
        let compute_graph = graph.build::<f64, f64>()?;

        let v = compute_graph.compute(&7.0);
        assert_eq!(v, 88.0);

        Ok(())
    }
}
