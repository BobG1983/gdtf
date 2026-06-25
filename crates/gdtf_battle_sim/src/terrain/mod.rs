//! The mutable battlefield grid/state: occupancy, surfaces, slabs, cover, vertical links,
//! the authored terrain-piece schema + registry (GTW-394), the per-tile ECS entity
//! layer (GTW-395), and the per-cell floor move-cost surface (GTW-396).

pub mod cover;
pub mod entity;
pub mod floor;
pub mod occupancy;
pub mod occupancy_sync;
pub mod piece;
pub mod slab;
pub mod surface;
pub mod vertical;
