//! Building and driving the battle app these tests run against.

use bevy::{app::App, prelude::MinimalPlugins, scene::ScenePlugin};
use cobalt_test_utils::unwatched_asset_plugin;
use gdtf_battle_sim::{
    battle::{BattleSimPlugin, SetupBattleRequested},
    ganger::GangRegistry,
    metric::{Cell, CellLevel, Level},
    rng::BattleSeed,
    situation::{PlacedGanger, Situation},
    test_support::{test_armor_registry, test_melee_weapon_registry, test_terrain_registry},
    tuning::{
        CombatTuning, ExitEmplacementTu, ReactionCapBase, ReactionCapPerReactions, ReactionPMax,
        ReactionPMin, ReactionTuning, SuppressionRadius, SuppressionStabilityPenalty, ViewRange,
    },
};

use super::test_ranged_registry;

pub(crate) const PLAYER: u8 = 0;

pub(crate) const TEST_VIEW_RANGE: u16 = 12;

pub(crate) fn ground(x: i32, y: i32) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(0))
}

/// Reaction tuning whose interrupt is certain rather than a roll: `p_min == p_max == 1.0`.
pub(crate) const fn forced_reaction_tuning(cap: u32) -> ReactionTuning {
    ReactionTuning {
        cap_base:            ReactionCapBase::new(cap as f32),
        cap_per_reactions:   ReactionCapPerReactions::new(0.0),
        p_min:               ReactionPMin::new(1.0),
        p_max:               ReactionPMax::new(1.0),
        suppression_radius:  SuppressionRadius::new(0),
        suppression_penalty: SuppressionStabilityPenalty::new(0.0),
    }
}

pub(crate) fn battle_app(seed: u64) -> (App, u64) {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, unwatched_asset_plugin(), ScenePlugin));
    app.add_plugins(BattleSimPlugin);
    app.insert_resource(CombatTuning {
        view_range: ViewRange::new(TEST_VIEW_RANGE),
        ..Default::default()
    });
    app.insert_resource(test_ranged_registry());
    app.insert_resource(test_melee_weapon_registry());
    app.insert_resource(test_armor_registry());
    app.insert_resource(test_terrain_registry());
    (app, seed)
}

pub(crate) fn drive_setup(
    app: &mut App,
    seed: u64,
    situation_and_gangs: (Situation, Vec<PlacedGanger>, GangRegistry),
) {
    let (situation, placements, gangs) = situation_and_gangs;
    app.world_mut().insert_resource(gangs);
    app.world_mut().write_message(SetupBattleRequested::new(
        situation,
        placements,
        BattleSeed::new(seed),
    ));
    for _ in 0..4 {
        app.update();
    }
}

pub(crate) fn step(app: &mut App, ticks: u32) {
    for _ in 0..ticks {
        app.update();
    }
}

/// Rewrite the exit-emplacement leaf, which [`battle_app`] otherwise leaves at its default.
pub(crate) fn set_exit_tu(app: &mut App, tu: u8) {
    let world = app.world_mut();
    let Some(mut tuning) = world.get_resource_mut::<CombatTuning>() else {
        unreachable!("battle_app inserts the combat tuning this harness reads back");
    };
    tuning.exit_emplacement_tu = ExitEmplacementTu::new(tu);
}
