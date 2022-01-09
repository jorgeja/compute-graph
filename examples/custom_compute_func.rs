use compute_graph::prelude::{Compute, Constant, Graph};

#[derive(Clone)]
struct Sum(f64);
impl Compute for Sum {
    type In = f64;
    type Out = f64;
    fn compute(&self, input: &[&Self::In]) -> Self::Out {
        input.iter().map(|v| *v).sum()
    }
}

fn main() {
    let mut graph = Graph::new();

    let sum_handle = graph.insert_node("sum", Sum(42.0));
    let const_handle = graph.insert_node("the_answer", Constant(42.0));

    graph.add_input(&sum_handle, &const_handle).unwrap();
    graph.connect_to_input(&sum_handle);
    graph.set_output_node(&sum_handle);

    let compute_graph = graph.build::<f64, f64>().unwrap();
    let value = compute_graph.compute(&1.0);
    assert_eq!(value, 43.0);
}
