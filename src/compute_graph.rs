use std::any::{Any, TypeId};
use std::marker::PhantomData;
use std::cell::RefCell;
use crate::compute::InnerCompute;
use crate::graph::Node;


#[derive(Clone)]
pub(crate) struct ComputeNode<'a> {
    pub(crate) inputs: Vec<usize>,
    pub(crate) func: Box<dyn InnerCompute + 'a>,
}

pub struct ComputeGraph<'a, In, Out> {
    outputs: Vec<RefCell<Box<dyn Any>>>,
    nodes: Vec<ComputeNode<'a>>,
    _in_type: PhantomData<In>,
    _out_type: PhantomData<Out>,
}

impl<'a, In, Out> ComputeGraph<'a, In, Out> 
where 
    In: Any + Copy,
    Out: Any + Copy 
{
    pub(crate) fn new(nodes: Vec<ComputeNode<'a>>) -> Self {
        let outputs = nodes.iter().map(|node| RefCell::new(node.func.init_output())).collect::<Vec<_>>();
        Self {
            outputs,
            nodes,
            _in_type: PhantomData,
            _out_type: PhantomData,
        }
    }

    pub fn compute(&self, input: &In) -> Out {                
        for (i, node) in self.nodes.iter().enumerate() {
            let mut output = self.outputs[i].borrow_mut();            
            if node.func.input_type() == TypeId::of::<()>(){
                node.func.inner_compute(&[], output.as_mut());
            } else if node.inputs.is_empty() {
                node.func.inner_compute(&[input], output.as_mut());
            } else {
                let inp = node.inputs.iter().map(|inp| self.outputs[*inp].borrow()).collect::<Vec<_>>();
                let inp_refs = inp.iter().map(|inp| inp.as_ref()).collect::<Vec<_>>();
                node.func.inner_compute(&inp_refs, output.as_mut());
            }
        }
        *self.outputs.last().unwrap().borrow().as_ref().downcast_ref::<Out>().unwrap()
    }
}

impl<'a, In, Out> Clone for ComputeGraph<'a, In, Out> 
where 
    In: Any + Copy,
    Out: Any + Copy
{
    fn clone(&self) -> Self {
        ComputeGraph::new(self.nodes.clone())
    }
}