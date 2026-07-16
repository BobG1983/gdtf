//! GTW-735: headless integration tests for the DIRECT actor-selection seam — pushing
//! `ActIntent::Select(Entity)` over the REAL `GdtfBattleInputPlugin` drain
//! (`dispatch_act_intents` → `SelectedShooter`).
//!
//! These drive the SAME drain the keyboard / action-bar / mouse surfaces feed; the `Select`
//! variant has no local producer yet (its producer is the GTW-694 T4 network inject path), so
//! the tests push it directly onto the shared `PendingActIntent` seam — the ONE write-point.
//!
//! Coverage:
//!
//! - ACCEPTANCE — selecting a player-faction ganger via the intent sets `SelectedShooter`.
//! - REJECTION — selecting an enemy-faction ganger is refused (the selection is unchanged),
//!   the SAME `faction == player` gate `decide_left_click`'s SELECT clause enforces.
//! - CHANGE-DETECTION HYGIENE — re-selecting the CURRENT shooter does not spuriously trip
//!   `Changed<SelectedShooter>` (asserted through the same `Res<SelectedShooter>::is_changed()`
//!   mechanism `sync_fire_mode_on_select` keys off).
//! - FAIL-CLOSED — a dead / despawned entity token is refused SAFELY (no panic; the existing
//!   valid selection is left intact).
//!
//! Every `app.world_mut()` mutation is in a TEST BODY — the accepted headless idiom
//! (`bevy-traps.md` #7 carve-out (a)). No function here takes `&mut World`/`&World`.

use bevy::{input::ButtonInput, prelude::*};
use gdtf_battle_input::{ActIntent, GdtfBattleInputPlugin, PendingActIntent, SelectedShooter};
use gdtf_battle_presenter::{ActiveLevel, ViewMode};
use gdtf_battle_sim::{
    battle::PlayerFaction,
    prelude::{BattleInProgress, Cell, CellLevel, Faction, Level, OccupancyGrid},
    test_support::GangerEntityBuilder,
    vertical::VerticalLinkGraph,
};

/// The faction the player controls (matches the inserted `PlayerFaction`).
const PLAYER_FACTION: Faction = Faction::new(0);
/// An ENEMY faction (distinct from [`PLAYER_FACTION`]) — a `Select` on its gangers is refused.
const ENEMY_FACTION: Faction = Faction::new(1);
/// The level all direct-select tests run on.
const LEVEL: Level = Level::new(0);

/// Counts the updates on which `SelectedShooter` tripped change-detection — the
/// change-detection-hygiene probe, reading the SAME `Res<SelectedShooter>::is_changed()` flag
/// `sync_fire_mode_on_select` gates on. Test scaffolding (out of the no-bare-types scope).
#[derive(Resource, Default)]
struct SelectionChangeCount(u32);

/// Records a change whenever `SelectedShooter` trips change-detection this update — registered in
/// `Last` so it runs AFTER the drain (in `Update`) every frame.
fn record_selection_changes(
    selected: Res<SelectedShooter>,
    mut count: ResMut<SelectionChangeCount>,
) {
    if selected.is_changed() {
        count.0 += 1;
    }
}

/// Builds the base headless direct-select app: `MinimalPlugins` plus `GdtfBattleInputPlugin`, the
/// presenter-owned `ActiveLevel` / `ViewMode`, the `BattleInProgress` gate (so the drain runs), an
/// empty `OccupancyGrid` + `VerticalLinkGraph` (so unrelated input systems validate), the
/// `PlayerFaction` the select gates on, seeded `ButtonInput<KeyCode>`, and the change-detection
/// probe in `Last`. Deliberately does NOT insert `ButtonInput<MouseButton>`, so the mouse-click
/// surfaces stay gated off (build.rs's `resource_exists::<ButtonInput<MouseButton>>` guard) and
/// only the intent seam under test drives `SelectedShooter`.
fn select_app() -> App {
    let mut app = App::new();
    // `update_selection_highlight` (in `GdtfBattleInputPlugin`) spawns its reticle via
    // `Commands::spawn_scene`, which needs an `AssetServer` + the scene schedule under
    // `MinimalPlugins` (the selection_cycle.rs precedent).
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        bevy::scene::ScenePlugin,
    ))
    .add_plugins(GdtfBattleInputPlugin);
    app.world_mut().insert_resource(ActiveLevel::new(LEVEL));
    app.world_mut().insert_resource(ViewMode::default());
    app.world_mut().insert_resource(BattleInProgress);
    app.world_mut().insert_resource(OccupancyGrid::default());
    app.world_mut()
        .insert_resource(VerticalLinkGraph::default());
    app.world_mut()
        .insert_resource(PlayerFaction::new(PLAYER_FACTION));
    app.world_mut()
        .insert_resource(ButtonInput::<KeyCode>::default());
    app.init_resource::<SelectionChangeCount>();
    app.add_systems(Last, record_selection_changes);
    app
}

/// Spawns a ganger of `faction` at the cell `(x, y)` on [`LEVEL`] and returns its entity. The
/// direct-select gate reads only the ganger's `&Faction`.
fn placed_ganger(app: &mut App, faction: Faction, x: i32, y: i32) -> Entity {
    GangerEntityBuilder::new()
        .faction(faction)
        .at(CellLevel::new(Cell::new(x, y), LEVEL))
        .spawn(app.world_mut())
}

/// Pushes an [`ActIntent`] onto the shared seam (the network-inject / keyboard write-point).
fn push(app: &mut App, intent: ActIntent) {
    app.world_mut()
        .resource_mut::<PendingActIntent>()
        .push(intent);
}

/// The current `SelectedShooter` entity, if any.
fn selection(app: &App) -> Option<Entity> {
    **app.world().resource::<SelectedShooter>()
}

/// ACCEPTANCE — a `Select` intent for a PLAYER-faction ganger sets `SelectedShooter` to it.
///
/// Starts with a DIFFERENT player ganger selected (so the GTW-255 auto-select — which only ever
/// fills an EMPTY selection — is inert), then `Select`s a second player ganger and asserts the
/// selection moved to exactly the carried entity. The target is NOT the deterministic-first
/// ganger, so the pick cannot be confused with auto-select.
#[test]
fn select_intent_selects_player_faction_ganger() {
    let mut app = select_app();
    let g_first = placed_ganger(&mut app, PLAYER_FACTION, 0, 0);
    let g_target = placed_ganger(&mut app, PLAYER_FACTION, 5, 5);
    app.world_mut()
        .insert_resource(SelectedShooter::new(g_first));

    push(&mut app, ActIntent::Select(g_target));
    app.update();

    assert_eq!(
        selection(&app),
        Some(g_target),
        "Select(player ganger) sets SelectedShooter to the carried entity",
    );
}

/// REJECTION — a `Select` intent for an ENEMY-faction ganger is refused; the selection is
/// unchanged (the SAME `faction == player` gate `decide_left_click`'s SELECT clause enforces).
#[test]
fn select_intent_refuses_enemy_faction_ganger() {
    let mut app = select_app();
    let g_player = placed_ganger(&mut app, PLAYER_FACTION, 0, 0);
    let g_enemy = placed_ganger(&mut app, ENEMY_FACTION, 5, 5);
    app.world_mut()
        .insert_resource(SelectedShooter::new(g_player));

    push(&mut app, ActIntent::Select(g_enemy));
    app.update();

    assert_eq!(
        selection(&app),
        Some(g_player),
        "Select(enemy ganger) is REFUSED — the player selection is left untouched",
    );
}

/// CHANGE-DETECTION HYGIENE — re-selecting the CURRENT shooter does not spuriously trip
/// `Changed<SelectedShooter>`.
///
/// Discriminating (verification.md Rule 2): if the drain wrote `SelectedShooter` unconditionally
/// (dropping the `*selected != next` guard), the probe would count a change and this FAILS.
#[test]
fn reselecting_current_shooter_is_change_detection_noop() {
    let mut app = select_app();
    let g_player = placed_ganger(&mut app, PLAYER_FACTION, 0, 0);
    app.world_mut()
        .insert_resource(SelectedShooter::new(g_player));

    // Settle the initial selection + its change tick, then zero the counter so we measure ONLY
    // the re-select update below.
    app.update();
    app.world_mut().resource_mut::<SelectionChangeCount>().0 = 0;

    // Re-select the SAME shooter — the drain must NOT rewrite SelectedShooter (change hygiene).
    push(&mut app, ActIntent::Select(g_player));
    app.update();

    assert_eq!(
        selection(&app),
        Some(g_player),
        "re-selecting the current shooter keeps the same selection",
    );
    assert_eq!(
        app.world().resource::<SelectionChangeCount>().0,
        0,
        "re-selecting the current shooter must not spuriously trip Changed<SelectedShooter>",
    );
}

/// FAIL-CLOSED — a `Select` intent for a DEAD / despawned entity token is refused SAFELY: no
/// panic (the deny-lints forbid `unwrap`/`expect`), and the existing valid selection is intact.
#[test]
fn select_intent_refuses_dead_entity_token() {
    let mut app = select_app();
    let g_keep = placed_ganger(&mut app, PLAYER_FACTION, 0, 0);
    let g_victim = placed_ganger(&mut app, PLAYER_FACTION, 5, 5);
    app.world_mut()
        .insert_resource(SelectedShooter::new(g_keep));
    // Despawn the target BEFORE the intent drains, so its token is dead when `select_target`
    // resolves it — the `factions` query lookup errors → the selection is refused.
    app.world_mut().despawn(g_victim);

    push(&mut app, ActIntent::Select(g_victim));
    app.update();

    assert_eq!(
        selection(&app),
        Some(g_keep),
        "Select(dead token) is REFUSED (fail-closed, no panic) — the selection is untouched",
    );
}
