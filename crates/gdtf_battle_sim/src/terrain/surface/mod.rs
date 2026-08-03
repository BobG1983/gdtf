//! Surface grid: present / destroyed / absent slabs and accrued ground damage.

mod grid;

#[cfg(test)]
mod test;

pub use grid::{DamageApplied, GroundDamage, SlabState, SurfaceGrid};
