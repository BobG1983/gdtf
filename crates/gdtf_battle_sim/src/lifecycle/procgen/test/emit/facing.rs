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
        piece::TerrainGraphicKey,
    },
};

const TURNED_WALL: TerrainUuid = TerrainUuid::new(Uuid::from_u128(0x0149_1179_0000_0001));
const STRAIGHT_WALL: TerrainUuid = TerrainUuid::new(Uuid::from_u128(0x0149_1179_0000_0002));
const FACING_FLOOR: TerrainUuid = TerrainUuid::new(Uuid::from_u128(0x0149_1179_0000_0003));

const AUTHORED_FACING: TerrainFacing = TerrainFacing::East;

fn facing_wall_def(key: TerrainUuid) -> TerrainDef {
    TerrainDef {
        key,
        display_name: TerrainDisplayName::new("Facing Test Wall".to_owned()),
        sim_kind: TerrainSimKind::Wall {
            hp:               CoverHp::new(40),
            armor_protection: ArmorProtection::new(6),
            armor_hardness:   ArmorHardness::new(3),
            height_band:      HeightBand::High,
        },
        presenter_kind: TerrainPresenterKind::Wall {
            graphic_name: TerrainGraphicKey::new("wall".to_owned()),
        },
        tags: Vec::new(),
        views: TerrainViews::new(Vec::new()),
        on_death: Vec::new(),

        blocks_pathing: None,
        blocks_los: None,
        leaves_behind: LeavesBehind::Nothing,
    }
}

fn facing_terrain_defs() -> TerrainDefRegistry {
    let floor = TerrainDef {
        key:            FACING_FLOOR,
        display_name:   TerrainDisplayName::new("Facing Test Floor".to_owned()),
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
        views:          TerrainViews::new(Vec::new()),
        on_death:       Vec::new(),

        blocks_pathing: None,
        blocks_los:     None,
        leaves_behind:  LeavesBehind::Nothing,
    };
    TerrainDefRegistry::new([
        (TURNED_WALL, facing_wall_def(TURNED_WALL)),
        (STRAIGHT_WALL, facing_wall_def(STRAIGHT_WALL)),
        (FACING_FLOOR, floor),
    ])
}

fn facing_prefabs(theme: ThemeUuid, fp: GridSize) -> PrefabRegistry {
    let both_walls = |role: SpawnRole, stem: &str| {
        Prefab::new(
            PrefabName::new(stem.to_owned()),
            PrefabSpec::new(
                theme,
                fp,
                role,
                vec![
                    TerrainPlacementEntry::new(TURNED_WALL, at(1, 1), AUTHORED_FACING),
                    TerrainPlacementEntry::new(STRAIGHT_WALL, at(2, 2), TerrainFacing::default()),
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
fn poured_spawns_carry_the_authored_facing_and_floors_take_the_default() {
    let theme = theme();
    let (Some(board), Some(fp)) = (size(40, 40), size(12, 12)) else {
        return;
    };
    let terrain_defs = facing_terrain_defs();
    let prefabs = facing_prefabs(theme, fp);
    let themes = UuidThemeRegistry::new([(
        theme,
        UuidThemeDef {
            key:           theme,
            display_name:  ThemeDisplayName::new("Facing Test Theme".to_owned()),
            default_floor: FACING_FLOOR,
            terrain:       vec![TURNED_WALL, STRAIGHT_WALL, FACING_FLOOR],
        },
    )]);
    let knobs = tuning(0.8, 49, 2);

    let mut rng = ProcgenRng::from_root(BattleSeed::new(0x1179_0001));
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
        "the generate must succeed for a prefab placing two walls: {:?}",
        result.as_ref().err(),
    );
    let Ok(emitted) = result else {
        return;
    };
    let situation = emitted.situation;

    // pour_prefab translates each cell by the placed prefab's origin, so key on the piece.
    let turned: Vec<_> = situation
        .walls
        .iter()
        .filter(|wall| wall.piece == TURNED_WALL)
        .collect();
    assert!(
        !turned.is_empty(),
        "the authored TURNED_WALL must reach situation.walls at least once",
    );
    assert!(
        turned.iter().all(|wall| wall.facing == AUTHORED_FACING),
        "every spawn of the East-facing entry must carry East, not the default: {turned:?}",
    );

    let straight: Vec<_> = situation
        .walls
        .iter()
        .filter(|wall| wall.piece == STRAIGHT_WALL)
        .collect();
    assert!(
        !straight.is_empty(),
        "the authored STRAIGHT_WALL must reach situation.walls at least once",
    );
    assert!(
        straight
            .iter()
            .all(|wall| wall.facing == TerrainFacing::default()),
        "the default-facing entry must stay default — a constant facing in pour_prefab \
         fails here: {straight:?}",
    );

    assert!(
        !situation.floors.is_empty(),
        "the board must leave dead space, so floor_region emits floors",
    );
    assert!(
        situation
            .floors
            .iter()
            .all(|floor| floor.facing == TerrainFacing::default()),
        "floor_region has no entry to read, so every synthesised floor takes the default",
    );
}
