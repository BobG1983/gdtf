//! Battle-space metric: the cubic-voxel coordinate system the shot pipeline
//! flies in (see `docs/combat/battle-space.md`).
//!
//! **One continuous cubic-voxel metric, no pixels anywhere in the model**: x/y
//! are ground-plane cells, z is height in *levels*, and **one sim unit is one
//! cell on x = one cell on y = one level on z** — cubic voxels over the 60×60×8
//! grid, so a 3D shot vector's angles are honest. A cell is `floor(pos.x)` /
//! `floor(pos.y)`; a z-level is `floor(pos.z)`. The view's pixel projection
//! never enters the model — these are sim-unit voxel coordinates; that
//! projection is the presenter's job (`gdtf_battle_presenter`).
//!
//! Positions are math vectors in this metric (Bevy re-exports glam as
//! `bevy::math`, so these are bevy's own vector types). The newtypes below name
//! each domain quantity so a cell can never be mistaken for a sim point and a
//! storey index can never be passed where a coordinate belongs.

mod coords;
#[cfg(test)]
mod test;

pub use coords::{
    Cell, CellDef, CellDistance, CellLevel, CellLevelDef, CellUnit, Level, MAX_LEVELS, SimPos,
    SimUnit, cell_center, pos_to_cell,
};
