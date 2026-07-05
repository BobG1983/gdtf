//! GTW-469 — NS/EW wall orientation classification through the real
//! `generate_level` loader/emit path.

use bevy::asset::uuid::Uuid;

use super::support::*;
use crate::{
    armor::{ArmorHardness, ArmorProtection},
    cover::{CoverHp, HeightBand},
    level::{
        GridSize, Prefab, PrefabName, PrefabRegistry, PrefabSpec, SpawnRole, TerrainPlacementEntry,
        ThemeDisplayName, ThemeUuid, UuidThemeDef, UuidThemeRegistry,
    },
    procgen::generate_level,
    rng::{BattleSeed, ProcgenRng},
    terrain::{
        def::{
            TerrainDef, TerrainDefRegistry, TerrainDisplayName, TerrainPresenterKind,
            TerrainSimKind, TerrainUuid,
        },
        piece::TerrainGraphicKey,
    },
};

/// The NS-orientation GTW-469 test wall piece (`sim_kind = Wall`, `graphic_name = "wall"`).
const WALL_NS_EW_NS: TerrainUuid = TerrainUuid::new(Uuid::from_u128(0x0149_0469_0000_0001));
/// The EW-orientation companion (`sim_kind = Wall`, `graphic_name = "wall_ew"`) — SAME sim
/// semantics as [`WALL_NS_EW_NS`], distinct only in its presenter graphic key.
const WALL_NS_EW_EW: TerrainUuid = TerrainUuid::new(Uuid::from_u128(0x0149_0469_0000_0002));
/// The walkable default-floor test piece (a `Slab` `sim_kind` in the new model) the GTW-469
/// orientation fixture nominates as its default floor.
const WALL_NS_EW_FLOOR: TerrainUuid = TerrainUuid::new(Uuid::from_u128(0x0149_0469_0000_0003));

/// A `Wall` terrain def with the given key + presenter graphic role; the structural stats are
/// IDENTICAL for the NS and EW orientations (orientation is presentation-only, so the sim half
/// is the same for both — C5).
fn orientation_wall_def(key: TerrainUuid, graphic: &str) -> TerrainDef {
    TerrainDef {
        key,
        display_name: TerrainDisplayName::new("Test Wall".to_owned()),
        sim_kind: TerrainSimKind::Wall {
            hp:               CoverHp::new(40),
            armor_protection: ArmorProtection::new(6),
            armor_hardness:   ArmorHardness::new(3),
            height_band:      HeightBand::High,
        },
        presenter_kind: TerrainPresenterKind::Wall {
            graphic_name: TerrainGraphicKey::new(graphic.to_owned()),
        },
        tags: Vec::new(),
        on_death: None,
    }
}

/// The GTW-469 fixture terrain registry: the NS wall, the EW wall, and a walkable floor.
fn orientation_terrain_defs() -> TerrainDefRegistry {
    let floor = TerrainDef {
        key:            WALL_NS_EW_FLOOR,
        display_name:   TerrainDisplayName::new("Test Floor".to_owned()),
        sim_kind:       TerrainSimKind::Slab {
            hp:               crate::slab::SlabHp::new(60),
            armor_protection: ArmorProtection::new(1),
            armor_hardness:   ArmorHardness::new(0),
        },
        presenter_kind: TerrainPresenterKind::Slab {
            graphic_name: TerrainGraphicKey::new("floor".to_owned()),
            footfall:     None,
        },
        tags:           Vec::new(),
        on_death:       None,
    };
    TerrainDefRegistry::new([
        (WALL_NS_EW_NS, orientation_wall_def(WALL_NS_EW_NS, "wall")),
        (
            WALL_NS_EW_EW,
            orientation_wall_def(WALL_NS_EW_EW, "wall_ew"),
        ),
        (WALL_NS_EW_FLOOR, floor),
    ])
}

/// A `PrefabRegistry` whose player + enemy prefabs (footprint `fp`) each place ONE NS wall and
/// ONE EW wall at distinct footprint-local cells, so the emit must translate + classify each.
fn orientation_prefabs(theme: ThemeUuid, fp: GridSize) -> PrefabRegistry {
    let both_walls = |role: SpawnRole, stem: &str| {
        Prefab::new(
            PrefabName::new(stem.to_owned()),
            PrefabSpec::new(
                theme,
                fp,
                role,
                vec![
                    TerrainPlacementEntry::new(WALL_NS_EW_NS, at(1, 1)),
                    TerrainPlacementEntry::new(WALL_NS_EW_EW, at(2, 2)),
                ],
            ),
        )
    };
    let mut prefabs = PrefabRegistry::default();
    prefabs.insert(both_walls(SpawnRole::Player, "player_pad"));
    prefabs.insert(both_walls(SpawnRole::Enemy, "enemy_pad"));
    prefabs
}

/// GTW-469 C4 / C5 — an NS-wall and an EW-wall `TerrainUuid` placed in a [`PrefabSpec`] BOTH
/// resolve through the REAL `generate_level` loader/emit path and emit into the
/// [`walls`](Situation::walls) list, classified identically.
///
/// The fixture registry holds two `sim_kind = Wall` defs — the NS wall (`graphic_name = "wall"`)
/// and the EW wall (`graphic_name = "wall_ew"`) — that differ ONLY in their presenter graphic key
/// (orientation is presentation-only; the sim semantics are IDENTICAL, C5). A prefab places one
/// of each, then the real `generate_level` classifies the placed pieces by their `sim_kind`. The
/// assertion proves BOTH UUIDs surface in the emitted `walls` list (C4: the EW-wall `TerrainUuid`
/// resolves through the loader/emit path) and that the EW wall is bucketed exactly like the NS
/// wall (C5: same `Wall` classification, no sim-side behavioural difference). Pin-discriminating:
/// re-keying the EW def to a `Slab` `sim_kind` would route it into `slabs`, failing this test.
#[test]
fn ns_and_ew_walls_both_emit_as_walls_through_the_loader() {
    let terrain_defs = orientation_terrain_defs();

    // C5 precondition: the two orientation defs are BOTH Wall (identical sim_kind) — the
    // difference is purely the presenter graphic_name.
    let (Some(ns), Some(ew)) = (
        terrain_defs.def(&WALL_NS_EW_NS),
        terrain_defs.def(&WALL_NS_EW_EW),
    ) else {
        return;
    };
    assert!(
        matches!(ns.sim_kind, TerrainSimKind::Wall { .. })
            && matches!(ew.sim_kind, TerrainSimKind::Wall { .. }),
        "both the NS and EW wall defs must be sim_kind = Wall (orientation is presentation-only)",
    );

    let theme = theme();
    // The player/enemy footprints must meet `MinPlayerSide` (>= 10), so use 12x12 on a 40x40
    // board (the size the sibling emit tests use); local cells (1,1) + (2,2) fit comfortably.
    let (Some(board), Some(fp)) = (size(40, 40), size(12, 12)) else {
        return;
    };
    let prefabs = orientation_prefabs(theme, fp);
    let themes = UuidThemeRegistry::new([(
        theme,
        UuidThemeDef {
            key:           theme,
            display_name:  ThemeDisplayName::new("Test Theme".to_owned()),
            default_floor: WALL_NS_EW_FLOOR,
            terrain:       vec![WALL_NS_EW_NS, WALL_NS_EW_EW, WALL_NS_EW_FLOOR],
        },
    )]);
    let knobs = tuning(0.8, 49, 2);

    let mut rng = ProcgenRng::from_root(BattleSeed::new(0x0469_4311));
    let result = generate_level(
        &prefabs,
        &themes,
        &terrain_defs,
        theme,
        board,
        &mut rng,
        &knobs,
    );
    assert!(
        result.is_ok(),
        "the generate must succeed for a prefab placing NS + EW walls: {:?}",
        result.as_ref().err(),
    );
    let Ok(emitted) = result else {
        return;
    };
    let situation = emitted.situation;

    // C4 / C5: BOTH the NS and the EW wall TerrainUuid resolved through the loader/emit path and
    // were classified into the walls list (the EW wall is bucketed exactly like the NS wall).
    assert!(
        situation.walls.iter().any(|w| w.piece == WALL_NS_EW_NS),
        "the NS-wall TerrainUuid must emit into the walls list (the Wall classification)",
    );
    assert!(
        situation.walls.iter().any(|w| w.piece == WALL_NS_EW_EW),
        "the EW-wall TerrainUuid must emit into the walls list, classified identically to the \
         NS wall (C4 — it resolves through the loader; C5 — same Wall sim semantics)",
    );
    // The EW wall must NOT have leaked into the slabs list (it is a Wall, not a Slab) — the pin
    // that re-keying it to a Slab sim_kind would catch.
    assert!(
        !situation.slabs.iter().any(|s| s.piece == WALL_NS_EW_EW),
        "the EW wall must classify as a Wall (walls list), never a Slab (C5)",
    );
}
