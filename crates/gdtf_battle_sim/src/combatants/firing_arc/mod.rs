//! Whether a target cell lies inside the firing arc.

mod arc;
#[cfg(test)]
mod test;

pub use arc::{TargetInArc, target_in_arc};
