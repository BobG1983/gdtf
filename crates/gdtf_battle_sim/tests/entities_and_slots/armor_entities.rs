//! Armor relationship entities: shot wear goes through piece integrity components.

use bevy::{
    app::App,
    asset::AssetPlugin,
    ecs::system::{RunSystemOnce, SystemState},
    prelude::{Commands, Entity, MinimalPlugins},
    scene::ScenePlugin,
};
use gdtf_battle_sim::{
    armor::{ArmorIntegrity, Wears},
    cover::CoverLedger,
    fire::{BattleGrids, FireOrder, ShooterQuery, StruckBodies, Volley, WieldedWeapons},
    ganger::{Aim, Aiming, Facing},
    injuries::{InjuryRegistry, InjuryTables},
    prelude::{Cell, Direction, Faction, Level, OccupancyGrid, Stance, StanceKind},
    resolve_and_apply::WoundRoll,
    resolve_coarse::ShotKind,
    situation::{BattleRegistries, BattleSetup, setup_battle},
    slab::{BraceStairCells, SlabLedger},
    surface::SurfaceGrid,
    test_support::{
        GangerSpawnBuilder, SituationBuilder, injury_rng, key, severity_rng, shot_rng, single_mode,
        test_armor_registry, test_melee_weapon_registry, test_weapon_registry,
    },
    tuning::{CombatTuning, GangerStatTuning},
};

const fn shooter_at() -> bevy::math::IVec2 {
    bevy::math::IVec2::new(5, 6)
}
const fn enemy_at() -> bevy::math::IVec2 {
    bevy::math::IVec2::new(9, 6)
}

fn battle_app() -> Option<(App, BattleSetup)> {
    let (situation, gangs) = SituationBuilder::new()
        .with_gangers([
            GangerSpawnBuilder::new()
                .at(key(shooter_at().x, shooter_at().y, 0))
                .faction(Faction::new(0))
                .facing(Facing::new(Direction::East))
                .stance(Stance::new(StanceKind::Standing))
                .aiming(Aiming::new(true))
                .aim(Aim::new(10.0))
                .build(),
            GangerSpawnBuilder::new()
                .at(key(enemy_at().x, enemy_at().y, 0))
                .faction(Faction::new(1))
                .stance(Stance::new(StanceKind::Standing))
                .build(),
        ])
        .build_with_gangs();

    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default(), ScenePlugin));
    let registry = test_weapon_registry();
    let melee = test_melee_weapon_registry();
    let armor = test_armor_registry();
    let terrain = gdtf_battle_sim::test_support::test_terrain_registry();
    let stat_tuning = GangerStatTuning::default();
    let fallback_floor_cost = gdtf_battle_sim::tuning::CombatTuning::default()
        .move_costs
        .open;
    let outcome = app
        .world_mut()
        .run_system_once(move |mut commands: Commands| {
            setup_battle(
                &situation,
                BattleRegistries::new(
                    &gangs,
                    &registry,
                    &melee,
                    &armor,
                    &stat_tuning,
                    Some(&terrain),
                ),
                fallback_floor_cost,
                &mut commands,
            )
        });
    assert!(outcome.is_ok(), "the one-shot setup system must run");
    let setup = outcome.ok().and_then(Result::ok);
    assert!(setup.is_some(), "setup_battle must succeed");
    let setup = setup?;
    app.update();
    app.update();
    Some((app, setup))
}

fn fire_once(app: &mut App, shooter: Entity, seed: u64) -> Volley {
    type FireQueries<'w, 's> = (
        ShooterQuery<'w, 's>,
        WieldedWeapons<'w, 's>,
        StruckBodies<'w, 's>,
    );
    let tuning = CombatTuning::default();
    let mut rng = shot_rng(seed);
    let mut sev_rng = severity_rng(seed);
    let mut injury_rng = injury_rng(seed);
    let occupancy = app
        .world()
        .get_resource::<OccupancyGrid>()
        .cloned()
        .unwrap_or_default();
    let surface = app
        .world()
        .get_resource::<SurfaceGrid>()
        .cloned()
        .unwrap_or_default();
    let mut cover = app
        .world()
        .get_resource::<CoverLedger>()
        .cloned()
        .unwrap_or_default();
    let mut slab = app
        .world()
        .get_resource::<SlabLedger>()
        .cloned()
        .unwrap_or_default();
    let mode = single_mode(0.2, 1);

    let mut state: SystemState<FireQueries> = SystemState::new(app.world_mut());
    let volley = {
        let world = app.world_mut();
        let Ok((mut shooters, mut arms, mut bodies)) = state.get_mut(world) else {
            return Volley {
                reports: Vec::new(),
                shots:   Vec::new(),
                splash:  Vec::new(),
            };
        };
        gdtf_battle_sim::fire::fire(
            FireOrder {
                shooter,
                mode: &mode,
                target_cell: Cell::new(enemy_at().x, enemy_at().y),
                target_level: Level::new(0),
            },
            &mut shooters,
            &mut arms,
            &mut bodies,
            BattleGrids {
                occupancy:   &occupancy,
                surface:     &surface,
                cover:       &mut cover,
                slab:        &mut slab,
                brace_cells: &BraceStairCells::empty(),
            },
            &mut rng,
            &mut WoundRoll {
                tuning:       &tuning,
                severity_rng: &mut sev_rng,
                tables:       &InjuryTables::default(),
                registry:     &InjuryRegistry::default(),
                injury_rng:   &mut injury_rng,
            },
        )
    };
    state.apply(app.world_mut());
    volley
}

fn min_piece_integrity(app: &App, ganger: Entity) -> Option<i32> {
    let wears = app.world().get::<Wears>(ganger)?;
    bevy::ecs::relationship::RelationshipTarget::iter(wears)
        .filter_map(|piece| app.world().get::<ArmorIntegrity>(piece).map(|c| **c))
        .min()
}

#[test]
fn a_landed_shot_wears_the_struck_piece_entity_integrity() {
    let Some((mut app, setup)) = battle_app() else {
        return;
    };
    assert_eq!(
        setup.occupants.len(),
        2,
        "the setup must spawn both the shooter and the enemy",
    );
    let (Some(shooter_p), Some(enemy_p)) = (setup.occupants.first(), setup.occupants.get(1)) else {
        return;
    };
    let shooter = shooter_p.occupant;
    let enemy = enemy_p.occupant;

    let before = min_piece_integrity(&app, enemy);
    assert!(
        before.is_some(),
        "the enemy must carry worn-piece integrity components (the Wears entities)",
    );

    let mut landed = false;
    for seed in 0..256u64 {
        let volley = fire_once(&mut app, shooter, seed);
        let hit_ganger = volley
            .reports
            .iter()
            .any(|r| matches!(r.kind, ShotKind::Ganger(e) if e == enemy));
        if hit_ganger {
            landed = true;
            break;
        }
    }
    assert!(
        landed,
        "across the seed sweep at least one volley must land on the enemy (a Ganger hit)",
    );

    let after = min_piece_integrity(&app, enemy);
    assert!(
        matches!((before, after), (Some(b), Some(a)) if a < b),
        "a landed shot must wear the struck piece ENTITY's ArmorIntegrity below its \
         spawned value (before={before:?} after={after:?}) — the wear went through the \
         related piece entity, not a WornArmor array slot",
    );
}

#[test]
fn same_seed_reproduces_a_byte_equal_volley_through_the_relationship() {
    const SEED: u64 = 0xA12_0323;

    let run = || {
        let (mut app, setup) = battle_app()?;
        let shooter = setup.occupants.first().map(|p| p.occupant)?;
        Some(fire_once(&mut app, shooter, SEED))
    };

    let (first, second) = (run(), run());
    assert!(
        first.is_some() && second.is_some(),
        "both seeded runs must build a battle + fire a volley",
    );
    assert_eq!(
        first, second,
        "the same battle seed must reproduce a byte-equal Volley through the \
         armor-relationship lookup (determinism held)",
    );
}
