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
            LeavesBehind, TerrainDef, TerrainDefRegistry, TerrainDisplayName, TerrainPresenterKind,
            TerrainSimKind, TerrainUuid, TerrainViews,
        },
        facing::TerrainFacing,
    },
};

const WALL_NS_EW_NS: TerrainUuid = TerrainUuid::new(Uuid::from_u128(0x0149_0469_0000_0001));
const WALL_NS_EW_EW: TerrainUuid = TerrainUuid::new(Uuid::from_u128(0x0149_0469_0000_0002));
const WALL_NS_EW_FLOOR: TerrainUuid = TerrainUuid::new(Uuid::from_u128(0x0149_0469_0000_0003));

fn orientation_wall_def(key: TerrainUuid) -> TerrainDef {
    TerrainDef {
        key,
        display_name: TerrainDisplayName::new("Test Wall".to_owned()),
        sim_kind: TerrainSimKind::Wall {
            hp:               CoverHp::new(40),
            armor_protection: ArmorProtection::new(6),
            armor_hardness:   ArmorHardness::new(3),
            height_band:      HeightBand::High,
        },
        presenter_kind: TerrainPresenterKind::Wall,
        tags: Vec::new(),
        views: TerrainViews::new(Vec::new()),
        on_death: Vec::new(),

        blocks_pathing: None,
        blocks_los: None,
        leaves_behind: LeavesBehind::Nothing,
    }
}

fn orientation_terrain_defs() -> TerrainDefRegistry {
    let floor = TerrainDef {
        key:            WALL_NS_EW_FLOOR,
        display_name:   TerrainDisplayName::new("Test Floor".to_owned()),
        sim_kind:       TerrainSimKind::Slab {
            hp:               crate::slab::SlabHp::new(60),
            armor_protection: ArmorProtection::new(1),
            armor_hardness:   ArmorHardness::new(0),
        },
        presenter_kind: TerrainPresenterKind::Slab { footfall: None },
        tags:           Vec::new(),
        views:          TerrainViews::new(Vec::new()),
        on_death:       Vec::new(),

        blocks_pathing: None,
        blocks_los:     None,
        leaves_behind:  LeavesBehind::Nothing,
    };
    TerrainDefRegistry::new([
        (WALL_NS_EW_NS, orientation_wall_def(WALL_NS_EW_NS)),
        (WALL_NS_EW_EW, orientation_wall_def(WALL_NS_EW_EW)),
        (WALL_NS_EW_FLOOR, floor),
    ])
}

fn orientation_prefabs(theme: ThemeUuid, fp: GridSize) -> PrefabRegistry {
    let both_walls = |role: SpawnRole, stem: &str| {
        Prefab::new(
            PrefabName::new(stem.to_owned()),
            PrefabSpec::new(
                theme,
                fp,
                role,
                vec![
                    TerrainPlacementEntry::new(WALL_NS_EW_NS, at(1, 1), TerrainFacing::default()),
                    TerrainPlacementEntry::new(WALL_NS_EW_EW, at(2, 2), TerrainFacing::default()),
                ],
            ),
        )
    };
    let mut prefabs = PrefabRegistry::default();
    prefabs.insert(both_walls(SpawnRole::Player, "player_pad"));
    prefabs.insert(both_walls(SpawnRole::Enemy, "enemy_pad"));
    prefabs
}

#[test]
fn ns_and_ew_walls_both_emit_as_walls_through_the_loader() {
    let terrain_defs = orientation_terrain_defs();

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

    assert!(
        situation.walls.iter().any(|w| w.piece == WALL_NS_EW_NS),
        "the NS-wall TerrainUuid must emit into the walls list (the Wall classification)",
    );
    assert!(
        situation.walls.iter().any(|w| w.piece == WALL_NS_EW_EW),
        "the EW-wall TerrainUuid must emit into the walls list, classified identically to the \
         NS wall (C4 — it resolves through the loader; C5 — same Wall sim semantics)",
    );
    assert!(
        !situation.slabs.iter().any(|s| s.piece == WALL_NS_EW_EW),
        "the EW wall must classify as a Wall (walls list), never a Slab (C5)",
    );
}
