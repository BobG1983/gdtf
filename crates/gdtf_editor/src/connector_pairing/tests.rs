use gdtf_battle_sim::{
    armor::{ArmorHardness, ArmorProtection},
    level::{GridHeight, GridLevels, GridSize, GridWidth, ThemeUuid},
    metric::{CellLevel, Level},
    prelude::Cell,
    slab::SlabHp,
    terrain::{
        def::{
            LeavesBehind, TerrainDef, TerrainDefRegistry, TerrainDisplayName, TerrainPresenterKind,
            TerrainSimKind, TerrainTag, TerrainUuid, TerrainViewArt, TerrainViews, owed_views_for,
        },
        entity::TerrainPieceKind,
        facing::TerrainFacing,
        piece::TerrainGraphicKey,
    },
};

use super::{PairingOutcome, apply_placement_with_pairing, is_stair};
use crate::{
    editor_map::{EditorMap, PaintedPiece},
    placement::ProposedPlacement,
};

fn theme() -> ThemeUuid {
    ThemeUuid::new(bevy::asset::uuid::Uuid::from_u128(0x0149_1490_0002))
}

const fn tu(n: u128) -> TerrainUuid {
    TerrainUuid::new(bevy::asset::uuid::Uuid::from_u128(n))
}

fn size() -> GridSize {
    GridSize::new(GridWidth::new(4), GridHeight::new(4), GridLevels::new(3))
        .unwrap_or_else(|_| GridSize::default())
}

fn slab_def(key: TerrainUuid, label: &str, sprite: &str, tags: Vec<TerrainTag>) -> TerrainDef {
    let views = owed_views_for(TerrainPieceKind::Slab, &tags)
        .iter()
        .map(|view| TerrainViewArt {
            view:   *view,
            sprite: TerrainGraphicKey::new(sprite.to_owned()),
        })
        .collect();
    TerrainDef {
        key,
        display_name: TerrainDisplayName::new(label.to_owned()),
        sim_kind: TerrainSimKind::Slab {
            hp:               SlabHp::new(120),
            armor_protection: ArmorProtection::new(6),
            armor_hardness:   ArmorHardness::new(3),
        },
        presenter_kind: TerrainPresenterKind::Slab { footfall: None },
        tags,
        views: TerrainViews::new(views),
        on_death: Vec::new(),

        blocks_pathing: None,
        blocks_los: None,
        leaves_behind: LeavesBehind::Nothing,
    }
}

const STAIR: TerrainUuid = tu(0x0d);
const PLAIN_SLAB: TerrainUuid = tu(0x0e);

// The two defs swap sprite keys against their tags, so only the tag tells them apart.
fn registry() -> TerrainDefRegistry {
    TerrainDefRegistry::new([
        (
            STAIR,
            slab_def(STAIR, "Deck Stair", "gantry", vec![TerrainTag::Stair]),
        ),
        (
            PLAIN_SLAB,
            slab_def(PLAIN_SLAB, "Deck Slab", "stair_ns_up", Vec::new()),
        ),
    ])
}

fn at(cell: Cell, level: u8) -> CellLevel {
    CellLevel::new(cell, Level::new(level))
}

#[test]
fn is_stair_reads_the_tag_not_the_sprite_key() {
    let reg = registry();
    assert!(
        is_stair(&reg, &STAIR),
        "the def carries TerrainTag::Stair, so it is a staircase however its sprite keys read",
    );
    assert!(
        !is_stair(&reg, &PLAIN_SLAB),
        "the def carries no stair tag, so a `stair_ns_up` sprite key must not make it one",
    );
}

#[test]
fn placing_up_connector_auto_places_down_pair_above() {
    let reg = registry();
    let th = theme();
    let mut map = EditorMap::new();
    let cell = Cell::new(1, 1);

    let placement = ProposedPlacement::new(at(cell, 0), STAIR, TerrainFacing::East);
    let outcome = apply_placement_with_pairing(&mut map, &reg, th, &placement, size());

    assert_eq!(
        outcome,
        PairingOutcome::PairPlaced {
            paired: STAIR,
            at:     at(cell, 1),
        },
        "painting a staircase at N auto-places the same tile at N+1",
    );
    assert_eq!(
        map.tile_at_level(at(cell, 0)),
        Some(PaintedPiece::new(STAIR, TerrainFacing::East)),
        "the painted staircase is at N",
    );
    assert_eq!(
        map.tile_at_level(at(cell, 1)),
        Some(PaintedPiece::new(STAIR, TerrainFacing::East)),
        "the second end is auto-placed at N+1 wearing the source placement's facing, never a \
         reset default",
    );
    assert_eq!(map.painted_count(), 2, "both endpoints are present");
}

#[test]
fn up_connector_on_top_storey_skips_pair_fail_closed() {
    let reg = registry();
    let th = theme();
    let mut map = EditorMap::new();
    let cell = Cell::new(2, 2);
    let placement = ProposedPlacement::new(at(cell, 2), STAIR, TerrainFacing::default());
    let outcome = apply_placement_with_pairing(&mut map, &reg, th, &placement, size());

    assert_eq!(
        outcome,
        PairingOutcome::PlacedPairSkipped,
        "a staircase on the top storey skips its second end (fail-closed)",
    );
    assert_eq!(
        map.tile_at_level(at(cell, 2)),
        Some(PaintedPiece::new(STAIR, TerrainFacing::default())),
        "the staircase still landed on the top storey",
    );
    assert_eq!(
        map.painted_count(),
        1,
        "only the painted tile — no pair above the top"
    );
}

#[test]
fn non_connector_placement_places_no_pair() {
    let reg = registry();
    let th = theme();
    let mut map = EditorMap::new();
    let cell = Cell::new(0, 0);
    let placement = ProposedPlacement::new(at(cell, 0), PLAIN_SLAB, TerrainFacing::default());
    let outcome = apply_placement_with_pairing(&mut map, &reg, th, &placement, size());
    assert_eq!(
        outcome,
        PairingOutcome::PlacedNoPair,
        "an untagged slab places no pair"
    );
    assert_eq!(map.painted_count(), 1, "only the placed tile — no pair");
}
