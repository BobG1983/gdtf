mod apply;
mod tick;

#[cfg(test)]
mod test;

pub use apply::{DotAfflicted, DotApplied, apply_dot};
pub use tick::{DotTicked, tick_dot};
