//! Every grid, ledger and registry a destroyed piece and its successor write.

use bevy::{
    ecs::system::SystemParam,
    prelude::{Res, ResMut},
};

use crate::{
    cover::CoverLedger,
    effects::on_death::TerrainOnDeathRegistry,
    occupancy::OccupancyGrid,
    slab::{BraceStairCells, SlabLedger},
    surface::SurfaceGrid,
    terrain::{def::TerrainDefRegistry, entity::TerrainIndex},
};

/// The world state [`replace_destroyed_piece`](super::replace_destroyed_piece) writes.
#[derive(SystemParam)]
pub struct SuccessorWrites<'w> {
    pub(super) occupancy: ResMut<'w, OccupancyGrid>,
    pub(super) surface:   ResMut<'w, SurfaceGrid>,
    pub(super) index:     Option<ResMut<'w, TerrainIndex>>,
    pub(super) defs:      Option<Res<'w, TerrainDefRegistry>>,
    pub(super) on_death:  Option<ResMut<'w, TerrainOnDeathRegistry>>,
    pub(super) cover:     Option<ResMut<'w, CoverLedger>>,
    pub(super) slabs:     Option<ResMut<'w, SlabLedger>>,
    pub(super) brace:     Option<Res<'w, BraceStairCells>>,
}
