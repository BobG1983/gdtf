use bevy::prelude::{Commands, Entity};

use super::super::terrain_resolve::ResolvedCoverPiece;
use crate::{
    cover::{CoverEntry, CoverLedger},
    occupancy::TerrainKind,
    situation::Situation,
    terrain::{
        emplacement::{
            EmplacementEntrySides, EmplacementFacing, EmplacementState, MountedWeaponKey,
        },
        entity::{BlocksPathfinding, BlocksVision, TerrainCell, TerrainIndexKey},
        openable::{OpenState, OpenableBlocking},
    },
};

pub(super) fn seed_cover_terrain(
    situation: &Situation,
    resolved_covers: Vec<ResolvedCoverPiece>,
    commands: &mut Commands,
) -> (Vec<(TerrainIndexKey, Entity)>, Vec<TerrainKind>) {
    let mut cover_ledger = CoverLedger::new();
    let mut terrain_pairs: Vec<(TerrainIndexKey, bevy::prelude::Entity)> = Vec::new();
    let mut occupancy_kinds: Vec<TerrainKind> =
        Vec::with_capacity(situation.walls.len() + situation.scatter.len());
    let mut resolved_covers_iter = resolved_covers.into_iter();
    for cover in situation.walls.iter().chain(situation.scatter.iter()) {
        let resolved = resolved_covers_iter
            .next()
            .unwrap_or_else(|| unreachable!("resolved_covers length matches covers length"));
        occupancy_kinds.push(TerrainKind::from(resolved.piece_kind));
        let entry = CoverEntry::seeded(
            resolved.max_hp,
            resolved.height_band,
            resolved.armor_protection,
            resolved.armor_hardness,
            resolved.piece_kind,
        );
        cover_ledger.insert(cover.at, entry);
        let entity = commands
            .spawn((
                TerrainCell::new(cover.at),
                cover.piece,
                cover.facing,
                resolved.piece_kind,
                entry.max_hp,
                entry.height_band,
                entry.armor_protection,
                entry.armor_hardness,
                resolved.graphic,
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
        if let Some(mounted_weapon) = &resolved.emplacement {
            commands.entity(entity).insert((
                EmplacementState::Vacant,
                MountedWeaponKey::new(mounted_weapon.clone()),
                EmplacementEntrySides::new(resolved.entry_sides.clone()),
                EmplacementFacing::new(cover.facing),
            ));
        }
        terrain_pairs.push((TerrainIndexKey::Cover(cover.at), entity));
    }
    commands.insert_resource(cover_ledger);
    (terrain_pairs, occupancy_kinds)
}
