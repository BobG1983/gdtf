//! Putting the successor in the cell, or clearing the cell for the kind that was destroyed.

use bevy::prelude::{Commands, MessageWriter};

use super::{
    components::SlabLeftOpen,
    plan::{LeftBehind, Plan, SuccessorCover, SuccessorSlab},
    writes::SuccessorWrites,
};
use crate::{
    cover::CoverEntry,
    effects::on_death::OnDeathEffect,
    metric::CellLevel,
    occupancy::TerrainKind,
    slab::SlabEntry,
    surface::SlabState,
    terrain::{
        emplacement::{
            EmplacementEntrySides, EmplacementFacing, EmplacementState, MountedWeaponKey,
        },
        entity::{
            BlocksPathfinding, BlocksVision, TerrainBrace, TerrainCell, TerrainIndexKey,
            TerrainPieceKind,
        },
        openable::{OpenState, OpenableBlocking},
        piece::LeftoverSprite,
    },
};

/// Carry out one destroyed piece's plan: spawn what it leaves, or clear its cell.
pub(super) fn apply(
    commands: &mut Commands,
    writes: &mut SuccessorWrites,
    opened: &mut MessageWriter<SlabLeftOpen>,
    plan: Plan,
) {
    let Plan {
        key,
        at,
        kind,
        left,
        on_death,
    } = plan;
    match left {
        LeftBehind::Nothing => clear_cell(writes, opened, key, at, kind),
        LeftBehind::Sprite(graphic) => {
            clear_cell(writes, opened, key, at, kind);
            commands.spawn((TerrainCell::new(at), LeftoverSprite::new(graphic)));
        }
        LeftBehind::Cover(successor) => spawn_cover(commands, writes, at, *successor),
        LeftBehind::Slab(successor) => spawn_slab(commands, writes, at, *successor),
    }
    rewrite_on_death(writes, key, on_death);
}

// Clear only the grid the destroyed kind lives in, and drop only that kind's index key.
fn clear_cell(
    writes: &mut SuccessorWrites,
    opened: &mut MessageWriter<SlabLeftOpen>,
    key: TerrainIndexKey,
    at: CellLevel,
    kind: TerrainPieceKind,
) {
    match kind {
        TerrainPieceKind::Slab => {
            writes.surface.set_slab(at, SlabState::Absent);
            opened.write(SlabLeftOpen::new(at));
        }
        TerrainPieceKind::Wall | TerrainPieceKind::Cover | TerrainPieceKind::Emplacement => {
            writes.occupancy.set_terrain(at, TerrainKind::Open);
        }
    }
    if let Some(index) = writes.index.as_mut() {
        index.remove(&key);
    }
}

// Spawn the successor wall, cover or emplacement the way `seed_cover_terrain` seeds one.
fn spawn_cover(
    commands: &mut Commands,
    writes: &mut SuccessorWrites,
    at: CellLevel,
    successor: SuccessorCover,
) {
    let SuccessorCover {
        piece,
        facing,
        resolved,
    } = successor;
    let entry = CoverEntry::seeded(
        resolved.max_hp,
        resolved.height_band,
        resolved.armor_protection,
        resolved.armor_hardness,
        resolved.piece_kind,
    );
    if let Some(ledger) = writes.cover.as_mut() {
        ledger.insert(at, entry);
    }
    let entity = commands
        .spawn((
            TerrainCell::new(at),
            piece,
            facing,
            resolved.piece_kind,
            entry.max_hp,
            entry.height_band,
            entry.armor_protection,
            entry.armor_hardness,
        ))
        .id();
    if *resolved.blocks_path {
        commands.entity(entity).insert(BlocksPathfinding);
    }
    if let Some(band) = resolved.occludes_vision {
        commands.entity(entity).insert(BlocksVision::new(band));
    }
    if let Some(band) = resolved.openable {
        commands.entity(entity).insert((
            OpenState::Closed,
            OpenableBlocking::new(band),
            BlocksPathfinding,
            BlocksVision::new(band),
        ));
    }
    if let Some(mounted_weapon) = resolved.emplacement {
        commands.entity(entity).insert((
            EmplacementState::Vacant,
            MountedWeaponKey::new(mounted_weapon),
            EmplacementEntrySides::new(resolved.entry_sides),
            EmplacementFacing::new(facing),
        ));
    }
    writes
        .occupancy
        .set_terrain(at, TerrainKind::from(resolved.piece_kind));
    if let Some(index) = writes.index.as_mut() {
        index.set(TerrainIndexKey::Cover(at), entity);
    }
}

// Spawn the successor slab the way `seed_slab_terrain` seeds one.
fn spawn_slab(
    commands: &mut Commands,
    writes: &mut SuccessorWrites,
    at: CellLevel,
    successor: SuccessorSlab,
) {
    let SuccessorSlab {
        piece,
        facing,
        resolved,
    } = successor;
    let entry = SlabEntry::seeded(
        resolved.max_hp,
        resolved.armor_protection,
        resolved.armor_hardness,
    );
    if let Some(ledger) = writes.slabs.as_mut() {
        ledger.insert(at, entry);
    }
    writes.surface.set_slab(at, SlabState::Present);
    let entity = commands
        .spawn((
            TerrainCell::new(at),
            piece,
            facing,
            TerrainPieceKind::Slab,
            resolved.max_hp,
            resolved.armor_protection,
            resolved.armor_hardness,
        ))
        .id();
    if let Some(footfall) = resolved.footfall {
        commands.entity(entity).insert(footfall);
    }
    if *resolved.blocks_path {
        commands.entity(entity).insert(BlocksPathfinding);
    }
    if let Some(band) = resolved.occludes_vision {
        commands.entity(entity).insert(BlocksVision::new(band));
    }
    if let Some(band) = resolved.openable {
        commands.entity(entity).insert((
            OpenState::Closed,
            OpenableBlocking::new(band),
            BlocksPathfinding,
            BlocksVision::new(band),
        ));
    }
    if writes
        .brace
        .as_ref()
        .is_some_and(|brace| brace.braces_slab_at(&at))
    {
        commands.entity(entity).insert(TerrainBrace);
    }
    if let Some(index) = writes.index.as_mut() {
        index.set(TerrainIndexKey::Slab(at), entity);
    }
}

// The cell's on-death entry now belongs to the successor def, or to no def at all.
fn rewrite_on_death(
    writes: &mut SuccessorWrites,
    key: TerrainIndexKey,
    on_death: Vec<OnDeathEffect>,
) {
    let Some(registry) = writes.on_death.as_mut() else {
        return;
    };
    if on_death.is_empty() {
        registry.remove(&key);
    } else {
        registry.insert(key, on_death);
    }
}
