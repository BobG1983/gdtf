use bevy::{
    platform::collections::HashSet,
    prelude::{Commands, Entity},
};

use super::super::terrain_resolve::ResolvedSlabPiece;
use crate::{
    metric::CellLevel,
    situation::Situation,
    slab::{BraceStairCells, SlabEntry, SlabLedger},
    surface::{SlabState, SurfaceGrid},
    terrain::{
        entity::{
            BlocksPathfinding, BlocksVision, TerrainBrace, TerrainCell, TerrainIndexKey,
            TerrainPieceKind,
        },
        openable::{OpenState, OpenableBlocking},
    },
    vertical::LinkKind,
};

pub(super) fn stair_cell_sets(situation: &Situation) -> (HashSet<CellLevel>, HashSet<CellLevel>) {
    let mut stair_cell_set = HashSet::new();
    for link in &situation.vertical_links {
        if matches!(link.kind, LinkKind::Stair { .. }) {
            stair_cell_set.insert(link.from);
            stair_cell_set.insert(link.to);
        }
    }

    let mut brace_stair_cells_set: bevy::platform::collections::HashSet<crate::metric::CellLevel> =
        bevy::platform::collections::HashSet::new();
    for link in &situation.vertical_links {
        if matches!(link.kind, LinkKind::Stair { .. }) {
            let lower = if link.from.z <= link.to.z {
                link.from
            } else {
                link.to
            };
            brace_stair_cells_set.insert(lower);
        }
    }
    (stair_cell_set, brace_stair_cells_set)
}

pub(super) fn seed_slab_terrain(
    situation: &Situation,
    resolved_slabs: &[ResolvedSlabPiece],
    brace_cells: &HashSet<CellLevel>,
    commands: &mut Commands,
) -> Vec<(TerrainIndexKey, Entity)> {
    let mut surface_grid = SurfaceGrid::new();
    let mut slab_ledger = SlabLedger::new();
    let mut terrain_pairs: Vec<(TerrainIndexKey, Entity)> = Vec::new();
    for (slab_spawn, resolved) in situation.slabs.iter().zip(resolved_slabs.iter()) {
        surface_grid.set_slab(slab_spawn.at, SlabState::Present);

        let slab_entry = SlabEntry::seeded(
            resolved.max_hp,
            resolved.armor_protection,
            resolved.armor_hardness,
        );
        slab_ledger.insert(slab_spawn.at, slab_entry);

        let slab_entity = commands
            .spawn((
                TerrainCell::new(slab_spawn.at),
                TerrainPieceKind::Slab,
                resolved.max_hp,
                resolved.armor_protection,
                resolved.armor_hardness,
                resolved.graphic.clone(),
            ))
            .id();
        if let Some(footfall) = resolved.footfall.clone() {
            commands.entity(slab_entity).insert(footfall);
        }

        if *resolved.blocks_path {
            commands.entity(slab_entity).insert(BlocksPathfinding);
        }
        if let Some(band) = resolved.occludes_vision {
            commands.entity(slab_entity).insert(BlocksVision::new(band));
        }
        if let Some(band) = resolved.openable {
            commands.entity(slab_entity).insert((
                OpenState::Closed,
                OpenableBlocking::new(band),
                BlocksPathfinding,
                BlocksVision::new(band),
            ));
        }

        if let Some(below) = cell_below(slab_spawn.at)
            && brace_cells.contains(&below)
        {
            commands.entity(slab_entity).insert(TerrainBrace);
        }

        terrain_pairs.push((TerrainIndexKey::Slab(slab_spawn.at), slab_entity));
    }
    commands.insert_resource(surface_grid);
    commands.insert_resource(slab_ledger);

    commands.insert_resource(BraceStairCells::new(brace_cells.clone()));

    terrain_pairs
}

fn cell_below(cell: crate::metric::CellLevel) -> Option<crate::metric::CellLevel> {
    let storey = (*cell.level()).checked_sub(1)?;
    Some(crate::metric::CellLevel::new(
        cell.cell(),
        crate::metric::Level::new(storey),
    ))
}
