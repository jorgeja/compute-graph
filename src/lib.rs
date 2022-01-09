mod com_graph;
mod compute;
mod graph;
mod operations;

pub mod prelude {
    pub use crate::compute::Compute;
    pub use crate::graph::{Graph, NodeHandle};
    pub use crate::operations::*;
}
