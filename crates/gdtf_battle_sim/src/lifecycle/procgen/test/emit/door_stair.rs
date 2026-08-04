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

const DOOR_NS: TerrainUuid = TerrainUuid::new(Uuid::from_u128(0x0149_0470_0000_0001));
const DOOR_EW: TerrainUuid = TerrainUuid::new(Uuid::from_u128(0x0149_0470_0000_0002));
const STAIR_NS_UP: TerrainUuid = TerrainUuid::new(Uuid::from_u128(0x0149_0470_0000_0003));
const STAIR_NS_DOWN: TerrainUuid = TerrainUuid::new(Uuid::from_u128(0x0149_0470_0000_0004));
const STAIR_EW_UP: TerrainUuid = TerrainUuid::new(Uuid::from_u128(0x0149_0470_0000_0005));
const STAIR_EW_DOWN: TerrainUuid = TerrainUuid::new(Uuid::from_u128(0x0149_0470_0000_0006));
const DOOR_STAIR_FLOOR: TerrainUuid = TerrainUuid::new(Uuid::from_u128(0x0149_0470_0000_0007));

fn door_def(key: TerrainUuid, graphic: &str) -> TerrainDef {
    TerrainDef {
        key,
        display_name: TerrainDisplayName::new("Test Door".to_owned()),
        sim_kind: TerrainSimKind::Wall {
            hp:               CoverHp::new(40),
            armor_protection: ArmorProtection::new(6),
            armor_hardness:   ArmorHardness::new(3),
            height_band:      HeightBand::High,
        },
        presenter_kind: TerrainPresenterKind::Wall {
            graphic_name: TerrainGraphicKey::new(graphic.to_owned()),
        },
        tags: vec![crate::terrain::def::TerrainTag::Openable],
        on_death: None,

        blocks_pathing: None,
        blocks_los: None,
    }
}

fn stair_def(key: TerrainUuid, graphic: &str) -> TerrainDef {
    TerrainDef {
        key,
        display_name: TerrainDisplayName::new("Test Stair".to_owned()),
        sim_kind: TerrainSimKind::Slab {
            hp:               crate::slab::SlabHp::new(120),
            armor_protection: ArmorProtection::new(4),
            armor_hardness:   ArmorHardness::new(2),
        },
        presenter_kind: TerrainPresenterKind::Slab {
            graphic_name: TerrainGraphicKey::new(graphic.to_owned()),
            footfall:     None,
        },
        tags: Vec::new(),
        on_death: None,

        blocks_pathing: None,
        blocks_los: None,
    }
}

fn door_stair_terrain_defs() -> TerrainDefRegistry {
    let floor = TerrainDef {
        key:            DOOR_STAIR_FLOOR,
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

        blocks_pathing: None,
        blocks_los:     None,
    };
    TerrainDefRegistry::new([
        (DOOR_NS, door_def(DOOR_NS, "door_ns")),
        (DOOR_EW, door_def(DOOR_EW, "door_ew")),
        (STAIR_NS_UP, stair_def(STAIR_NS_UP, "stair_ns_up")),
        (STAIR_NS_DOWN, stair_def(STAIR_NS_DOWN, "stair_ns_down")),
        (STAIR_EW_UP, stair_def(STAIR_EW_UP, "stair_ew_up")),
        (STAIR_EW_DOWN, stair_def(STAIR_EW_DOWN, "stair_ew_down")),
        (DOOR_STAIR_FLOOR, floor),
    ])
}

fn door_stair_prefabs(theme: ThemeUuid, fp: GridSize) -> PrefabRegistry {
    let all_six = |role: SpawnRole, stem: &str| {
        Prefab::new(
            PrefabName::new(stem.to_owned()),
            PrefabSpec::new(
                theme,
                fp,
                role,
                vec![
                    TerrainPlacementEntry::new(DOOR_NS, at(1, 1)),
                    TerrainPlacementEntry::new(DOOR_EW, at(2, 1)),
                    TerrainPlacementEntry::new(STAIR_NS_UP, at(3, 1)),
                    TerrainPlacementEntry::new(STAIR_NS_DOWN, at(4, 1)),
                    TerrainPlacementEntry::new(STAIR_EW_UP, at(5, 1)),
                    TerrainPlacementEntry::new(STAIR_EW_DOWN, at(6, 1)),
                ],
            ),
        )
    };
    let mut prefabs = PrefabRegistry::default();
    prefabs.insert(all_six(SpawnRole::Player, "player_pad"));
    prefabs.insert(all_six(SpawnRole::Enemy, "enemy_pad"));
    prefabs
}

#[test]
fn door_and_stair_tiles_emit_through_the_loader_classified_by_kind() {
    let terrain_defs = door_stair_terrain_defs();

    for door in [DOOR_NS, DOOR_EW] {
        let Some(def) = terrain_defs.def(&door) else {
            return;
        };
        assert!(
            matches!(def.sim_kind, TerrainSimKind::Wall { .. }),
            "a door must be sim_kind = Wall (a closed door blocks like a wall)",
        );
        assert!(
            def.tags
                .contains(&crate::terrain::def::TerrainTag::Openable),
            "a door must carry the sim-owned Openable tag (C5)",
        );
    }
    for stair in [STAIR_NS_UP, STAIR_NS_DOWN, STAIR_EW_UP, STAIR_EW_DOWN] {
        let Some(def) = terrain_defs.def(&stair) else {
            return;
        };
        assert!(
            matches!(def.sim_kind, TerrainSimKind::Slab { .. }),
            "a stair must be sim_kind = Slab (a walkable surface; vertical traversal is)",
        );
    }

    let theme = theme();
    let (Some(board), Some(fp)) = (size(40, 40), size(12, 12)) else {
        return;
    };
    let prefabs = door_stair_prefabs(theme, fp);
    let themes = UuidThemeRegistry::new([(
        theme,
        UuidThemeDef {
            key:           theme,
            display_name:  ThemeDisplayName::new("Test Theme".to_owned()),
            default_floor: DOOR_STAIR_FLOOR,
            terrain:       vec![
                DOOR_NS,
                DOOR_EW,
                STAIR_NS_UP,
                STAIR_NS_DOWN,
                STAIR_EW_UP,
                STAIR_EW_DOWN,
                DOOR_STAIR_FLOOR,
            ],
        },
    )]);
    let knobs = tuning(0.8, 49, 2);

    let mut rng = ProcgenRng::from_root(BattleSeed::new(0x0470_4311));
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
        "the generate must succeed for a prefab placing all 6 door/stair tiles: {:?}",
        result.as_ref().err(),
    );
    let Ok(emitted) = result else {
        return;
    };
    let situation = emitted.situation;

    for door in [DOOR_NS, DOOR_EW] {
        assert!(
            situation.walls.iter().any(|w| w.piece == door),
            "the door {door:?} must emit into the walls list (Wall classification, C4/C5)",
        );
        assert!(
            !situation.slabs.iter().any(|s| s.piece == door),
            "a door (Wall) must never classify as a Slab (C5)",
        );
    }
    for stair in [STAIR_NS_UP, STAIR_NS_DOWN, STAIR_EW_UP, STAIR_EW_DOWN] {
        assert!(
            situation.slabs.iter().any(|s| s.piece == stair),
            "the stair {stair:?} must emit into the slabs list (Slab classification, C4/C5)",
        );
        assert!(
            !situation.walls.iter().any(|w| w.piece == stair),
            "a stair (Slab) must never classify as a Wall (C5)",
        );
    }
}
