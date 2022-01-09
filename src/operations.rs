use crate::compute::Compute;
use std::{
    any::Any,
    marker::PhantomData,
    ops::{Add, Mul, Sub},
};

#[derive(Clone, Copy, Default)]
pub struct Constant<T>(pub T);
impl<T> Compute for Constant<T>
where
    T: Any + Copy + Default,
{
    type In = ();
    type Out = T;
    fn compute(&self, _: &[&Self::In]) -> Self::Out {
        self.0
    }
}

#[derive(Clone, Copy, Default)]
pub struct AddInputs<In> {
    _intype: PhantomData<In>,
}
impl<T> AddInputs<T> {
    pub fn new() -> Self {
        Self {
            _intype: PhantomData,
        }
    }
}

impl<T> Compute for AddInputs<T>
where
    T: Add<Output = T> + Any + Copy + Default,
{
    type In = T;
    type Out = T;
    fn compute(&self, inputs: &[&Self::In]) -> Self::Out {
        inputs.iter().fold(Self::In::default(), |acc, &v| *v + acc)
    }
}

#[derive(Clone, Copy, Default)]
pub struct SubInputs<In> {
    _intype: PhantomData<In>,
}
impl<T> SubInputs<T> {
    pub fn new() -> Self {
        Self {
            _intype: PhantomData,
        }
    }
}

impl<T> Compute for SubInputs<T>
where
    T: Sub<Output = T> + Any + Copy + Default,
{
    type In = T;
    type Out = T;
    fn compute(&self, inputs: &[&Self::In]) -> Self::Out {
        inputs.iter().fold(Self::In::default(), |acc, &v| *v - acc)
    }
}

#[derive(Clone, Copy, Default)]
pub struct MulInputs<T> {
    _intype: PhantomData<T>,
}
impl<T> MulInputs<T> {
    pub fn new() -> Self {
        Self {
            _intype: PhantomData,
        }
    }
}

impl<T> Compute for MulInputs<T>
where
    T: Mul<Output = T> + Any + Copy + Default,
{
    type In = T;
    type Out = T;
    fn compute(&self, inputs: &[&Self::In]) -> Self::Out {
        if inputs.len() == 1 {
            *inputs[0]
        } else {
            inputs.iter().skip(1).fold(*inputs[0], |prod, &v| *v * prod)
        }
    }
}
