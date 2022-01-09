use crate::compute::InnerCompute;
use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::marker::PhantomData;

#[derive(Clone)]
pub(crate) struct ComputeNode {
    pub(crate) connected_to_input: bool,
    pub(crate) inputs: Vec<usize>,
    pub(crate) func: Box<dyn InnerCompute + 'static>,
}

pub struct ComputeGraph<In, Out> {
    outputs: Vec<RefCell<Box<dyn Any>>>,
    nodes: Vec<ComputeNode>,
    _intype: PhantomData<In>,
    _outtype: PhantomData<Out>,
}

impl<In, Out> ComputeGraph<In, Out> {
    pub(crate) fn new(nodes: Vec<ComputeNode>) -> Self {
        let outputs = nodes
            .iter()
            .map(|node| RefCell::new(node.func.init_output()))
            .collect::<Vec<_>>();
        Self {
            outputs,
            nodes,
            _intype: PhantomData,
            _outtype: PhantomData,
        }
    }

    pub fn compute(&self, input: &In) -> Out
    where
        In: Any + Copy,
        Out: Any + Copy,
    {
        for (i, node) in self.nodes.iter().enumerate() {
            let mut output = self.outputs[i].borrow_mut();
            if node.func.input_type() == TypeId::of::<()>() {
                node.func.inner_compute(&[], output.as_mut());
            } else {
                let inp = node
                    .inputs
                    .iter()
                    .map(|inp| self.outputs[*inp].borrow())
                    .collect::<Vec<_>>();

                let mut inp_refs = inp.iter().map(|inp| inp.as_ref()).collect::<Vec<_>>();

                if node.connected_to_input {
                    inp_refs.push(input);
                }

                node.func.inner_compute(&inp_refs, output.as_mut());
            }
        }
        *self
            .outputs
            .last()
            .unwrap()
            .borrow()
            .as_ref()
            .downcast_ref::<Out>()
            .unwrap()
    }
}

impl<In, Out> Clone for ComputeGraph<In, Out> {
    fn clone(&self) -> Self {
        ComputeGraph::new(self.nodes.clone())
    }
}
