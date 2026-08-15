//! Battle map: occupancy, surfaces, cover, floors, openables, slabs, pieces.

/// Cover pieces and ledgers.
pub mod cover;
/// Terrain definitions and registry.
pub mod def;
/// Emplacements and manning state.
pub mod emplacement;
/// Per-tile terrain ECS entities.
pub mod entity;
/// Cardinal facing of a placed piece.
pub mod facing;
/// Floor cost grid.
pub mod floor;
/// Occupancy grid and blocking projections.
pub mod occupancy;
/// Keep occupancy in sync with moves, deaths, and destruction.
pub mod occupancy_sync;
/// Openable doors and hatches.
pub mod openable;
/// Authored terrain pieces.
pub mod piece;
/// Slabs and surface damage.
pub mod slab;
/// Surface grid resource.
pub mod surface;
/// Vertical links between levels.
pub mod vertical;
