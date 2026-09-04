use bevy::asset::uuid::Uuid;
use gdtf_battle_sim::{
    armor::{ArmorHardness, ArmorProtection},
    cover::{CoverHp, HeightBand},
    level::{GridHeight, GridLevels, GridSize, GridWidth, ThemeUuid},
    metric::{Cell, CellLevel, Level},
    slab::SlabHp,
    terrain::{
        def::{
            LeavesBehind, TerrainDef, TerrainDefRegistry, TerrainDisplayName, TerrainPresenterKind,
            TerrainSimKind, TerrainUuid, TerrainViews,
        },
        facing::TerrainFacing,
    },
};

use super::super::command::paint_one;
use crate::{
    editor_map::{EditorMap, PaintedPiece},
    mcp::wire::PlacementVerdictNet,
    placement::{PlacementVerdict, ProposedPlacement},
    session::MapEditorSession,
};

const LADDER: TerrainUuid = TerrainUuid::new(Uuid::from_u128(0x1AD4));
const SLAB: TerrainUuid = TerrainUuid::new(Uuid::from_u128(0x51AB));

fn theme() -> ThemeUuid {
    ThemeUuid::new(Uuid::from_u128(0x7EA3))
}

fn size() -> GridSize {
    match GridSize::new(GridWidth::new(4), GridHeight::new(4), GridLevels::new(3)) {
        Ok(size) => size,
        Err(fault) => unreachable!("4x4x3 sits inside every grid bound: {fault}"),
    }
}

// A ladder is a display name, not a kind: `classify` reads the name and nothing else.
fn ladder_def() -> TerrainDef {
    TerrainDef {
        key:            LADDER,
        display_name:   TerrainDisplayName::new("Steel Ladder".to_owned()),
        sim_kind:       TerrainSimKind::Cover {
            hp:               CoverHp::new(20),
            armor_protection: ArmorProtection::new(2),
            armor_hardness:   ArmorHardness::new(1),
            height_band:      HeightBand::Low,
        },
        presenter_kind: TerrainPresenterKind::Cover,
        tags:           Vec::new(),
        views:          TerrainViews::new(Vec::new()),
        on_death:       Vec::new(),
        blocks_pathing: None,
        blocks_los:     None,
        leaves_behind:  LeavesBehind::Nothing,
    }
}

fn slab_def() -> TerrainDef {
    TerrainDef {
        key:            SLAB,
        display_name:   TerrainDisplayName::new("Deck Slab".to_owned()),
        sim_kind:       TerrainSimKind::Slab {
            hp:               SlabHp::new(50),
            armor_protection: ArmorProtection::new(6),
            armor_hardness:   ArmorHardness::new(3),
        },
        presenter_kind: TerrainPresenterKind::Slab { footfall: None },
        tags:           Vec::new(),
        views:          TerrainViews::new(Vec::new()),
        on_death:       Vec::new(),
        blocks_pathing: None,
        blocks_los:     None,
        leaves_behind:  LeavesBehind::Nothing,
    }
}

fn registry() -> TerrainDefRegistry {
    TerrainDefRegistry::new([(LADDER, ladder_def()), (SLAB, slab_def())])
}

fn session() -> MapEditorSession {
    MapEditorSession::new(theme(), None, size())
}

#[test]
fn a_ladder_under_a_slab_reports_the_clear_the_write_is_about_to_do() {
    let cell = Cell::new(1, 1);
    let ground = CellLevel::new(cell, Level::new(0));
    let above = CellLevel::new(cell, Level::new(1));
    let mut map = EditorMap::new();
    assert!(
        map.paint_at(above, SLAB, TerrainFacing::North, size()),
        "the case needs a slab one storey up, or the placement would clear nothing",
    );

    let placement = ProposedPlacement::new(ground, LADDER, TerrainFacing::default());
    let reply = paint_one(&mut map, &session(), &registry(), &placement);

    assert_eq!(
        reply.verdict,
        Some(PlacementVerdictNet::from_verdict(
            &PlacementVerdict::legal_clearing(above)
        )),
        "the verdict is read before the write, so it names the slot the placement is about to \
         empty. Reading it afterwards answers a plain Legal, because the clear has already \
         happened and there is no slab left to report: {reply:?}",
    );
    assert!(
        map.tile_at_level(above).is_none(),
        "the write then does that clear, so the slot the verdict named is empty afterwards",
    );
    assert_eq!(
        map.tile_at_level(ground),
        Some(PaintedPiece::new(LADDER, TerrainFacing::default())),
        "the ladder landed in the slot the caller asked for",
    );
}
