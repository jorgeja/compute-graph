extern crate compute_graph;
use compute_graph::prelude::*;

fn main() {
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
    graph.add_input(&add_handle, &const_handle).unwrap();
    graph.add_input(&mul_handle, &const_handle).unwrap();

    //By default the graph sends the input to any input that has no other inputs
    //If you want a node to have the input together with other nodes you have to manually assign it
    graph.connect_to_input(&mul_handle);

    //We can specify an output node:
    graph.set_output_node(&add_handle);
    //We must build a ComputeGraph before we can compute anything
    //Graph fails if input type does not match output type, or there are cycles in the graph.
    let compute_graph = graph.build::<f64, f64>().unwrap();
    let v = compute_graph.compute(&7.0);
    assert_eq!(v, 336.0);

    //If we want to compute just part of the graph we can specify a node:
    let sub_compute_graph = graph.build_for_node::<f64, f64>(&mul_handle).unwrap();
    let v = sub_compute_graph.compute(&7.0);
    assert_eq!(v, 294.0);

    //Lets replace the constant value. Will fail if the input/output types don't match up
    graph.replace_node(&const_handle, Constant(11.0)).unwrap();

    //We must rebuild the graph after manipulating inputs/edges
    let compute_graph = graph.build::<f64, f64>().unwrap();

    let v = compute_graph.compute(&7.0);
    assert_eq!(v, 88.0);
}
