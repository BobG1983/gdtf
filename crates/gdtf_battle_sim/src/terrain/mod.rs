//! The mutable battlefield grid/state: occupancy, surfaces, slabs, cover, vertical links,
//! the authored terrain-piece schema + registry (GTW-394), and the per-tile ECS entity
//! layer (GTW-395).

pub mod cover;
pub mod entity;
pub mod occupancy;
pub mod occupancy_sync;
pub mod piece;
pub mod slab;
pub mod surface;
pub mod vertical;
