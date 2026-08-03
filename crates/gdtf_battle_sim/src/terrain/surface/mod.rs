//! Persistent surface grid: the model's authoritative store of floor/roof **slab**
//! stays destroyed and ground damage accrues (a model-authoritative store in the
mod grid;

#[cfg(test)]
mod test;

pub use grid::{DamageApplied, GroundDamage, SlabState, SurfaceGrid};
