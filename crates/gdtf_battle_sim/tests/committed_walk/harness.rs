//! Shared GTW-355 walk fixture: the live battle-app driver, the situations, the walk-state
//! readers, and the walk-completion driver.

use bevy::{
    app::App,
    asset::AssetPlugin,
    prelude::{Entity, MinimalPlugins},
    scene::ScenePlugin,
};
use gdtf_battle_sim::{
    acts::movement::WalkInProgress,
    battle::{BattleSimPlugin, SetupBattleRequested},
    floor::FloorCostGrid,
    ganger::{GangRegistry, Speed},
    metric::{Cell, CellLevel, Level},
    occupancy::OccupancyGrid,
    pathfinder::{PlanningView, find_path},
    prelude::{Faction, Position, Stance, StanceKind, Tu},
    rng::BattleSeed,
    situation::Situation,
    test_support::{
        GangerSpawnBuilder, SituationBuilder, test_armor_registry, test_melee_weapon_registry,
        test_weapon_registry,
    },
    tuning::{CombatTuning, ReactionCapBase, ReactionCapPerReactions, ReactionTuning, ViewRange},
    vertical::VerticalLinkGraph,
    visibility::{FactionRelation, SquadVisibility},
};

/// An arbitrary (not shipped tuning) seed for the test battle's RNG stream.
pub(crate) const SEED: u64 = 0x5A1C_AC75;

/// Gang `0` is the player.
pub(crate) const PLAYER: u8 = 0;
/// Gang `1` is the enemy.
pub(crate) const ENEMY: u8 = 1;

/// A view range that LIGHTS the local field a few cells out (so a short walk routes), yet
/// is short enough that a cell several tiles away is UNSEEN at spawn. Arbitrary test
/// tuning, never a pinned shipped magnitude.
pub(crate) const TEST_VIEW_RANGE: u16 = 4;

/// A ground-floor `(cell, level)` key.
pub(crate) fn ground(x: i32, y: i32) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(0))
}

/// The player ganger's spawn cell.
pub(crate) fn player_at() -> CellLevel {
    ground(5, 5)
}

/// Build the FULL live-runtime harness (the GTW-354 `battle_app` idiom): `MinimalPlugins`,
/// `AssetPlugin`, `ScenePlugin`, and `BattleSimPlugin`, with the persistent `Load`
/// resources a `MinimalPlugins` app has no `AssetServer` to load, and a short view range.
pub(crate) fn battle_app() -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default(), ScenePlugin));
    app.add_plugins(BattleSimPlugin);
    app.insert_resource(CombatTuning {
        view_range: ViewRange::new(TEST_VIEW_RANGE),
        // GTW-468: the LIVE reaction-fire trigger now interrupts a walking actor that steps
        // into an opposing watcher's LOS. These walk tests ISOLATE the GTW-355 stop-on-reveal
        // / bump-stop / §48 mechanics, so reaction fire is DISABLED here (a zero cap →
        // `may_interrupt` is always false → no interrupt ever fires) to keep them testing
        // exactly the walk mechanic they were written for. The reaction trigger's own
        // end-to-end coverage is `tests/reaction_trigger/`. (The stop-on-interrupt
        // test still drives a SYNTHETIC ReactionShotFired, which the disabled trigger does not
        // affect.)
        reaction: ReactionTuning {
            cap_base: ReactionCapBase::new(0.0),
            cap_per_reactions: ReactionCapPerReactions::new(0.0),
            ..Default::default()
        },
        ..Default::default()
    });
    app.insert_resource(test_weapon_registry());
    // GTW-505: the melee registry (with the `fists` default) so each ganger's melee
    // weapon resolves at setup (fixture gangers author none -> `fists`).
    app.insert_resource(test_melee_weapon_registry());
    app.insert_resource(test_armor_registry());
    app
}

/// Drive a setup through the REAL `setup_battle_on_request` Ok path and settle it (the
/// deferred `bsn!` ganger scenes materialize and the first-run recompute fills the fog).
pub(crate) fn drive_setup(app: &mut App, situation_and_gangs: (Situation, GangRegistry)) {
    let (situation, gangs) = situation_and_gangs;
    // GTW-414: insert the synthesized GangRegistry so setup_battle_on_request resolves
    // the PlacedGanger (gang, member) refs (the weapon/armor registries' precedent).
    app.world_mut().insert_resource(gangs);
    app.world_mut()
        .write_message(SetupBattleRequested::new(situation, BattleSeed::new(SEED)));
    app.update();
    app.update();
    app.update();
}

/// The player ganger entity (gang 0), found via a `world_mut()` query.
pub(crate) fn player_entity(app: &mut App) -> Option<Entity> {
    let world = app.world_mut();
    let mut query = world.query::<(Entity, &Faction)>();
    query
        .iter(world)
        .find(|(_, faction)| ***faction == PLAYER)
        .map(|(entity, _)| entity)
}

/// The current `(Position, Tu)` of `entity`.
pub(crate) fn pos_and_tu(app: &App, entity: Entity) -> Option<(CellLevel, u8)> {
    let position = app.world().get::<Position>(entity).map(|p| **p)?;
    let tu = app.world().get::<Tu>(entity).map(|t| **t)?;
    Some((position, tu))
}

/// Whether `entity` still carries a `WalkInProgress` (the walk is in flight).
pub(crate) fn is_walking(app: &App, entity: Entity) -> bool {
    app.world().get::<WalkInProgress>(entity).is_some()
}

/// The `find_path` total TU for the route from `start` to `goal`, computed over the LIVE
/// world resources through the EXACT production planning path (the squad fog + a player-
/// relative occupant resolver) — so it is the same total `dispatch_move` gated on.
pub(crate) fn plan_total(app: &App, start: CellLevel, goal: CellLevel) -> Option<u8> {
    let grid = app.world().get_resource::<OccupancyGrid>()?;
    let links = app.world().get_resource::<VerticalLinkGraph>()?;
    let squad = app.world().get_resource::<SquadVisibility>()?;
    let tuning = app.world().get_resource::<CombatTuning>()?;
    // GTW-396: read the FloorCostGrid the dispatch uses for its per-step costs.
    let floor_costs = app.world().get_resource::<FloorCostGrid>()?;
    // The player squad sees its own gang; any non-player occupant is Other (the
    // dispatch_move `relation_to` shape, player-relative).
    let planning = PlanningView::new(squad, |_occupant| FactionRelation::Other);
    let path = find_path(
        start,
        goal,
        grid,
        links,
        tuning,
        floor_costs,
        gdtf_battle_sim::injuries::MovementCostFactor::IDENTITY,
        &planning,
    )
    .ok()?;
    Some(*path.total())
}

/// Drive `app.update()` to dispatch a freshly-written `MoveRequested` and then walk it to
/// completion — at least one tick (so the accept starts the walk) and then until the
/// `WalkInProgress` is gone (the walk finished or stopped) or a generous tick budget is
/// exhausted (no open-ended loop). The first tick both dispatches the request AND takes
/// the first step (`advance_walk` runs `.after(dispatch_move)` in the same band).
pub(crate) fn run_until_walk_ends(app: &mut App, entity: Entity) {
    for _ in 0..32 {
        app.update();
        if !is_walking(app, entity) {
            return;
        }
    }
}

/// A one-player situation: a standing player ganger (gang 0) at [`player_at`] with the
/// given Speed (GTW-384: the derived TU budget = `tu_base + tu_per_speed·Speed`, so a
/// high Speed gives an ample TU pool — the absolute starting TU is read from the world,
/// the walk tests assert TU SPENT relative to it, never a pinned start).
pub(crate) fn one_player_situation(speed: f32) -> (Situation, GangRegistry) {
    SituationBuilder::new()
        .with_gangers([GangerSpawnBuilder::new()
            .at(player_at())
            .faction(Faction::new(PLAYER))
            .stance(Stance::new(StanceKind::Standing))
            .speed(Speed::new(speed))
            .build()])
        .build_with_gangs()
}

/// A player + a far enemy situation: the player at [`player_at`], an enemy (gang 1) at
/// `enemy_cell` (placed beyond [`TEST_VIEW_RANGE`] so it is UNSEEN at spawn). Both get
/// the given Speed (→ an ample derived TU pool, GTW-384).
pub(crate) fn player_and_enemy_situation(
    speed: f32,
    enemy_cell: CellLevel,
) -> (Situation, GangRegistry) {
    SituationBuilder::new()
        .with_gangers([
            GangerSpawnBuilder::new()
                .at(player_at())
                .faction(Faction::new(PLAYER))
                .stance(Stance::new(StanceKind::Standing))
                .speed(Speed::new(speed))
                .build(),
            GangerSpawnBuilder::new()
                .at(enemy_cell)
                .faction(Faction::new(ENEMY))
                .stance(Stance::new(StanceKind::Standing))
                .speed(Speed::new(speed))
                .build(),
        ])
        .build_with_gangs()
}
