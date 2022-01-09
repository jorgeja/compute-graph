use dyn_clone::DynClone;
use std::any::{Any, TypeId};

pub trait Compute: Clone {
    type In;
    type Out;
    fn compute(&self, inputs: &[&Self::In]) -> Self::Out
    where
        Self::In: Any + Copy + Default,
        Self::Out: Any + Copy + Default;
}

impl<OuterIn, OuterOut> Compute for fn(&[&OuterIn]) -> OuterOut
where
    OuterIn: Any + Copy + Default,
    OuterOut: Any + Copy + Default,
{
    type In = OuterIn;
    type Out = OuterOut;
    fn compute(&self, inputs: &[&Self::In]) -> Self::Out {
        self(inputs)
    }
}

pub(crate) trait InnerCompute: DynClone {
    fn init_output(&self) -> Box<dyn Any>;
    fn input_type(&self) -> TypeId;
    fn output_type(&self) -> TypeId;
    fn inner_compute(&self, inputs: &[&dyn Any], output: &mut dyn Any);
}
dyn_clone::clone_trait_object!(InnerCompute);

impl<T, InnerIn, InnerOut> InnerCompute for T
where
    T: Compute<In = InnerIn, Out = InnerOut>,
    InnerIn: Any + Copy + Default + 'static,
    InnerOut: Any + Copy + Default + 'static,
{
    fn init_output(&self) -> Box<dyn Any> {
        Box::new(InnerOut::default())
    }
    fn input_type(&self) -> TypeId {
        TypeId::of::<InnerIn>()
    }
    fn output_type(&self) -> TypeId {
        TypeId::of::<InnerOut>()
    }
    fn inner_compute(&self, inputs: &[&dyn Any], output: &mut dyn Any) {
        let inputs = inputs
            .iter()
            .map(|a| a.downcast_ref::<InnerIn>().unwrap())
            .collect::<Vec<_>>();
        let output_val = output.downcast_mut::<InnerOut>().unwrap();
        *output_val = self.compute(&inputs);
    }
}
