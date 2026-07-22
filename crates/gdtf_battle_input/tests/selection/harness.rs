//! Shared selection fixture: the headless selection app, occupancy / hover
//! authoring, the intent push, and the selection / highlight probes.

use bevy::{asset::AssetPlugin, input::ButtonInput, prelude::*, scene::ScenePlugin};
use gdtf_battle_input::{
    ActIntent, GdtfBattleInputPlugin, PendingActIntent, SelectedShooter, SelectionHighlight,
};
use gdtf_battle_presenter::{ActiveLevel, ViewMode};
use gdtf_battle_sim::{
    battle::PlayerFaction,
    ganger::LifeState,
    prelude::{BattleInProgress, CellLevel, Faction, Level, OccupancyGrid},
    vertical::VerticalLinkGraph,
};

/// The faction the player controls in these tests (matches `PlayerFaction`).
pub(crate) const PLAYER_FACTION: Faction = Faction::new(0);
/// An ENEMY faction (distinct from [`PLAYER_FACTION`]).
pub(crate) const ENEMY_FACTION: Faction = Faction::new(1);

/// Mints a valid throwaway [`Entity`] id without a panic (`Entity` has no public
/// numeric constructor in 0.18.1; spawning into a scratch world yields a real id).
pub(crate) fn mint_entity() -> Entity {
    World::new().spawn_empty().id()
}

/// Builds a focused headless selection app: `MinimalPlugins` + the
/// `GdtfBattleInputPlugin`, the presenter-owned `ActiveLevel`, the `BattleInProgress`
/// gate, an empty `OccupancyGrid`, `CombatTuning`, an empty `ButtonInput<MouseButton>`,
/// and the `PlayerFaction` the GTW-238 click decision gates on. (The keybind table is
/// asset-loaded, so under `MinimalPlugins` no `Keybinds` resolves — the keyboard systems
/// simply do not run; the level/intent-queue tests push intents directly.)
pub(crate) fn selection_app(active_level: Level) -> App {
    use gdtf_battle_sim::tuning::CombatTuning;
    let mut app = App::new();
    // GTW-322: this builder inserts both `BattleInProgress` + `OccupancyGrid`, so
    // `update_selection_highlight` runs and spawns its reticle via `Commands::spawn_scene`,
    // which PANICS under `MinimalPlugins` without an `AssetServer` + the scene schedule (the
    // spike-documented requirement). The inert (no-`BattleInProgress`) builders below leave
    // the system gated off, so they keep the bare `MinimalPlugins` harness.
    app.add_plugins((MinimalPlugins, AssetPlugin::default(), ScenePlugin))
        .add_plugins(GdtfBattleInputPlugin);
    app.world_mut()
        .insert_resource(ActiveLevel::new(active_level));
    // GTW-521 — `dispatch_act_intents` also mutates the presenter-owned `ViewMode`.
    app.world_mut().insert_resource(ViewMode::default());
    app.world_mut().insert_resource(BattleInProgress);
    app.world_mut().insert_resource(OccupancyGrid::default());
    app.world_mut().insert_resource(CombatTuning::default());
    app.world_mut()
        .insert_resource(PlayerFaction::new(PLAYER_FACTION));
    // GTW-356: the shared left-click decision reads `Res<VerticalLinkGraph>` (the OQ-4
    // link-tile gate) via `LeftClickReads`, and `battle_act_gate()` now gates the click systems
    // on it — seed an empty graph so the SELECT / CLEAR / NoOp click decision runs.
    app.world_mut()
        .insert_resource(VerticalLinkGraph::default());
    app.world_mut()
        .insert_resource(ButtonInput::<MouseButton>::default());
    app
}

/// Spawns a PLAYER-faction occupant at `cell` (so the GTW-238 SELECT branch picks it)
/// and returns its entity. Without a real `Faction` matching `PlayerFaction` the unified
/// left-click decision would not select it.
pub(crate) fn place_player_ganger(app: &mut App, cell: CellLevel) -> Entity {
    let ganger = app.world_mut().spawn(PLAYER_FACTION).id();
    app.world_mut()
        .resource_mut::<OccupancyGrid>()
        .set_occupant(cell, Some(ganger));
    ganger
}

/// Spawns a PLAYER-faction occupant carrying `life` at `cell` and seeds occupancy — the
/// GTW-729 fixture for the click SELECT life gate. A `LifeState::Downed` / `Dead` occupant is
/// NOT a selectable ACTOR (the click leaves the acting selection untouched), while a `Downed`
/// ally stays a stabilize TARGET the contextual panel offers.
pub(crate) fn place_player_ganger_with_life(
    app: &mut App,
    cell: CellLevel,
    life: LifeState,
) -> Entity {
    let ganger = app.world_mut().spawn((PLAYER_FACTION, life)).id();
    app.world_mut()
        .resource_mut::<OccupancyGrid>()
        .set_occupant(cell, Some(ganger));
    ganger
}

/// Sets the `InspectTarget`'s live hovered cell to a given cell (or clears it).
pub(crate) fn set_hovered(app: &mut App, cell: Option<CellLevel>) {
    use gdtf_battle_input::InspectTarget;
    app.world_mut().insert_resource(InspectTarget::new(cell));
}

/// The current `SelectedShooter` value.
pub(crate) fn selected(app: &App) -> Option<Entity> {
    app.world()
        .get_resource::<SelectedShooter>()
        .and_then(|s| **s)
}

/// The single selection-highlight sprite's translation + visibility, if it exists.
pub(crate) fn highlight_state(app: &mut App) -> Option<(Vec3, Visibility)> {
    let mut q = app
        .world_mut()
        .query_filtered::<(&Transform, &Visibility), With<SelectionHighlight>>();
    q.iter(app.world()).next().map(|(t, v)| (t.translation, *v))
}

/// Counts the selection-highlight sprites in the world.
pub(crate) fn highlight_count(app: &mut App) -> usize {
    let mut q = app
        .world_mut()
        .query_filtered::<Entity, With<SelectionHighlight>>();
    q.iter(app.world()).count()
}

/// Pushes an intent onto the shared `PendingActIntent` queue from the test body —
/// the same `push` the keyboard / `gdtf_app` button surfaces call.
pub(crate) fn push_intent(app: &mut App, intent: ActIntent) {
    app.world_mut()
        .resource_mut::<PendingActIntent>()
        .push(intent);
}
