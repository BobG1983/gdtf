//! [`seed_cover_terrain`] — phases 2 + 2.5 of [`setup_battle`](super::setup_battle):
//! seed the [`CoverLedger`] + spawn one terrain entity per cover piece
//! (blocking / openable / emplacement tags).

use bevy::prelude::{Commands, Entity};

use super::super::terrain_resolve::ResolvedCoverPiece;
use crate::{
    cover::{CoverEntry, CoverLedger},
    occupancy::TerrainKind,
    situation::Situation,
    terrain::{
        emplacement::{EmplacementState, MountedWeaponKey},
        entity::{BlocksPathfinding, BlocksVision, TerrainCell, TerrainIndexKey},
        openable::{OpenState, OpenableBlocking},
    },
};

/// Phases 2 + 2.5 — seed the cover ledger from walls + scatter (the one unified
/// ledger) and (GTW-395/396) spawn ONE terrain entity per cover piece carrying the
/// static stats + presentation hooks (`TerrainGraphicKey` / `FootfallSound`). The
/// `resolved_covers` vec is in walls-then-scatter order, mirroring the
/// `situation.walls.chain(situation.scatter)` iteration order inside.
///
/// Returns the accumulated `(TerrainIndexKey, Entity)` terrain pairs plus each
/// piece's occupancy [`TerrainKind`] (walls-then-scatter order) for the
/// orchestrator's grid pour.
pub(super) fn seed_cover_terrain(
    situation: &Situation,
    resolved_covers: Vec<ResolvedCoverPiece>,
    commands: &mut Commands,
) -> (Vec<(TerrainIndexKey, Entity)>, Vec<TerrainKind>) {
    let mut cover_ledger = CoverLedger::new();
    let mut terrain_pairs: Vec<(TerrainIndexKey, bevy::prelude::Entity)> = Vec::new();
    // GTW-483: capture each cover piece's occupancy TerrainKind here — derived from the
    // resolved SPEC VARIANT (`resolved.piece_kind`), NOT from which authoring list the
    // piece sat in. A Cover/Scatter spec authored in the `walls` list (or a Wall spec in
    // `scatter`) therefore reads its DEF's own kind into the occupancy grid. In
    // walls-then-scatter order, mirroring the placement-build zip below.
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
        );
        cover_ledger.insert(cover.at, entry);
        // GTW-491: spawn the terrain entity for this cover piece. The entity carries STATIC
        // stats (the max HP ceiling + band + armor) plus the presentation graphic. The live
        // HP pool stays authoritative in the CoverLedger. Commands::spawn is the bevy-traps #7
        // form (never world.spawn inside a registered system).
        //
        // NET-NEW (GTW-491): `resolved.graphic` (a TerrainGraphicKey) is now attached to
        // cover entities for ALL kinds — INCLUDING Wall (a wall carried NO graphic on the old
        // model). GTW-493 (T07c presenter) reads it. Footfall is no longer attached to a
        // cover/wall entity (it is Slab-only in the new model — the def's presenter Wall/Cover
        // variants carry no footfall field).
        let entity = commands
            .spawn((
                TerrainCell::new(cover.at),
                resolved.piece_kind,
                entry.max_hp,           // CoverHp — the static max (now Component)
                entry.height_band,      // HeightBand — the static band (now Component)
                entry.armor_protection, // ArmorProtection — already Component
                entry.armor_hardness,   // ArmorHardness — already Component
                resolved.graphic, // TerrainGraphicKey — presenter resolves to atlas entry (NET-NEW for Wall)
            ))
            .id();
        // GTW-501 C1/D2: attach the BlocksPathfinding marker when the def derives
        // path-blocking (a Wall/Cover blocks by default → `resolved.blocks_path` is true, so
        // existing walls/cover keep blocking with no content migration). `Added` fires on
        // this insert, so the GTW-501 projection picks it up the next time it runs (a unit
        // struct is not a Bundle in 0.19, so insert it conditionally rather than tupling).
        if resolved.blocks_path {
            commands.entity(entity).insert(BlocksPathfinding);
        }
        // GTW-502 C1/C2: attach the BlocksVision component (carrying its height-aware band)
        // when the def derives vision occlusion — for a Wall/Cover the kind default makes this
        // Some(its band), so existing walls/cover keep occluding sight at the SAME band their
        // CoverLedger entry already does (zero regression / idempotent re-block). `Added` fires
        // on this insert, so the GTW-502 projection picks it up the next time it runs (a band
        // payload makes BlocksVision a single-component, not a Bundle, so insert it
        // conditionally rather than tupling).
        if let Some(band) = resolved.occludes_vision {
            commands.entity(entity).insert(BlocksVision::new(band));
        }
        // GTW-503 C1/C2: an Openable Wall/Cover (a door) is attached OpenState::Closed +
        // OpenableBlocking(band) and FORCED to carry BOTH blocking components at the closed
        // band — GTW-503 owns the openable's blocking lifecycle. For a Wall/Cover this is
        // idempotent with the GTW-501/502 inserts above (blocks_path is already true and the
        // band already matches), but it is asserted here so a closed door blocks both even if a
        // future kind-default ever changed; the band is the same one its CoverLedger entry uses.
        if let Some(band) = resolved.openable {
            commands.entity(entity).insert((
                OpenState::Closed,
                OpenableBlocking::new(band),
                BlocksPathfinding,
                BlocksVision::new(band),
            ));
        }
        // GTW-543: an Emplacement (a cover-like smashable structure resolved through the cover
        // path) is attached EmplacementState::Vacant + the MountedWeaponKey recorded from the
        // def, so the enter/exit toggle (apply_emplacement_toggle) can flip its state + Phase 2
        // can spawn the mounted gun WITHOUT re-reading the registry. Its BlocksPathfinding +
        // BlocksVision(band) + CoverLedger entry are ALREADY attached above (blocks_path /
        // occludes_vision derive true for Emplacement, and the entry was seeded like a cover)
        // — an empty emplacement is still a cover-like structure.
        if let Some(mounted_weapon) = &resolved.emplacement {
            commands.entity(entity).insert((
                EmplacementState::Vacant,
                MountedWeaponKey::new(mounted_weapon.clone()),
            ));
        }
        terrain_pairs.push((TerrainIndexKey::Cover(cover.at), entity));
    }
    commands.insert_resource(cover_ledger);
    (terrain_pairs, occupancy_kinds)
}
