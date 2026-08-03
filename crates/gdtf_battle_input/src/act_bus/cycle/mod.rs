//! The DATA-DRIVEN cyclic-act order (GTW-225 / GTW-48 S8 AC8): the fixed authored
//! ladder), not tuning magnitudes — so they live as authored `const` arrays read
mod orders;

#[cfg(test)]
mod test;

pub use orders::{FACING_CYCLE, STANCE_CYCLE, next_facing, next_stance};
