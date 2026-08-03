//! Fixed authored order for cyclic acts (facing and stance ladders).
mod orders;

#[cfg(test)]
mod test;

pub use orders::{FACING_CYCLE, STANCE_CYCLE, next_facing, next_stance};
