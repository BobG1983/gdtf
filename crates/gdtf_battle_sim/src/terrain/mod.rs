//! The mutable battlefield grid/state: occupancy, surfaces, slabs, cover, vertical links,
//! the authored terrain-piece schema + registry (GTW-394), the per-tile ECS entity
//! layer (GTW-395), the per-cell floor move-cost surface (GTW-396), and the unified
//! UUID-keyed terrain-definition model (GTW-484, [`def`]).

pub mod cover;
pub mod def;
pub mod entity;
pub mod floor;
pub mod occupancy;
pub mod occupancy_sync;
pub mod piece;
pub mod slab;
pub mod surface;
pub mod vertical;
