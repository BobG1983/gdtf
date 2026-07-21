//! GTW-458: headless integration tests for the Prev/Next selection-cycle seam — pushing
//! `ActIntent::SelectNext` / `SelectPrev` over the REAL `GdtfBattleInputPlugin` drain
//! (`dispatch_act_intents` → `SelectedShooter`), and the `Tab` / `Shift+Tab` keyboard surface
//! (`cycle_selection_keys` → the same intents).
//!
//! Tests are headless `GdtfBattleInputPlugin` apps: spawn player-faction (+ enemy)
//! `Faction`/`Position` gangers, push the cycle intent (or press the key), `app.update()`, and
//! assert `SelectedShooter` advances/wraps in `(z, y, x)` order, ignores enemies, and makes a
//! first selection from `None`.
//!
//! - C2 — `SelectNext` / `SelectPrev` cycle through the player gang in deterministic
//!   `(z, y, x)` order, WRAPPING, ignoring enemy gangers, and making a first selection from
//!   `None`.
//! - C3 — `Tab` pushes `SelectNext` and `Shift+Tab` pushes `SelectPrev` via the SAME intent seam.
//! - Empty player gang → no-op.
//!
//! Every `app.world_mut()` mutation is in a TEST BODY — the accepted headless idiom
//! (`bevy-traps.md` #7 carve-out (a)). No function here takes `&mut World`/`&World`.

use bevy::{input::ButtonInput, prelude::*};
use gdtf_battle_input::{
    ActIntent, BoundKey, GdtfBattleInputPlugin, Keybinds, PendingActIntent, SelectedShooter,
};
use gdtf_battle_presenter::{ActiveLevel, ViewMode};
use gdtf_battle_sim::{
    battle::PlayerFaction,
    ganger::LifeState,
    prelude::{BattleInProgress, Cell, CellLevel, Faction, Level, OccupancyGrid},
    test_support::GangerEntityBuilder,
    vertical::VerticalLinkGraph,
};
use gdtf_test_utils::{clear_keys, press_key};

/// The faction the player controls (matches the inserted `PlayerFaction`).
const PLAYER_FACTION: Faction = Faction::new(0);
/// An ENEMY faction (distinct from [`PLAYER_FACTION`]) — the cycle must SKIP its gangers.
const ENEMY_FACTION: Faction = Faction::new(1);
/// The level all cycle tests run on.
const LEVEL: Level = Level::new(0);

/// A fully-bound [`Keybinds`] table for the keyboard cycle test — `select_next` / `select_prev`
/// bound to `Tab` (the GTW-458 chord). Built in the test body (not parsed from the editable
/// shipped `.ron`), the `acts.rs` `test_keybinds` precedent.
const fn test_keybinds() -> Keybinds {
    Keybinds {
        select_clear:     BoundKey::KeyEscape,
        level_up:         BoundKey::KeyPageUp,
        level_down:       BoundKey::KeyPageDown,
        // GTW-521 — the full-view toggle key.
        toggle_full_view: BoundKey::KeyV,
        stance_cycle:     BoundKey::KeyC,
        aim_toggle:       BoundKey::KeyF,
        facing_cycle:     BoundKey::KeyR,
        select_next:      BoundKey::KeyTab,
        select_prev:      BoundKey::KeyTab,
    }
}

/// Builds the base headless cycle app: `MinimalPlugins` plus `GdtfBattleInputPlugin`, the
/// presenter-owned `ActiveLevel`, the `BattleInProgress` gate (so the drain + the keyboard band
/// run), an empty `OccupancyGrid` + `VerticalLinkGraph` (so unrelated input systems validate),
/// the `PlayerFaction` the cycle considers, and seeded `ButtonInput<KeyCode>`.
fn cycle_app() -> App {
    let mut app = App::new();
    // `update_selection_highlight` (in `GdtfBattleInputPlugin`) spawns its reticle via
    // `Commands::spawn_scene`, which needs an `AssetServer` + the scene schedule under
    // `MinimalPlugins` (the control.rs precedent).
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        bevy::scene::ScenePlugin,
    ))
    .add_plugins(GdtfBattleInputPlugin);
    app.world_mut().insert_resource(ActiveLevel::new(LEVEL));
    // GTW-521 — `dispatch_act_intents` also mutates the presenter-owned `ViewMode`.
    app.world_mut().insert_resource(ViewMode::default());
    app.world_mut().insert_resource(BattleInProgress);
    app.world_mut().insert_resource(OccupancyGrid::default());
    app.world_mut()
        .insert_resource(VerticalLinkGraph::default());
    app.world_mut()
        .insert_resource(PlayerFaction::new(PLAYER_FACTION));
    app.world_mut()
        .insert_resource(ButtonInput::<KeyCode>::default());
    app
}

/// Spawns a ganger of `faction` at the cell `(x, y)` on [`LEVEL`] and returns its entity. The
/// cycle reads only `(Entity, &Faction, &Position)`.
fn placed_ganger(app: &mut App, faction: Faction, x: i32, y: i32) -> Entity {
    GangerEntityBuilder::new()
        .faction(faction)
        .at(CellLevel::new(Cell::new(x, y), LEVEL))
        .spawn(app.world_mut())
}

/// Spawns a DOWNED ganger of `faction` at the cell `(x, y)` on [`LEVEL`] and returns its
/// entity — the GTW-729 fixture the cycle must SKIP (never land on).
fn placed_downed_ganger(app: &mut App, faction: Faction, x: i32, y: i32) -> Entity {
    GangerEntityBuilder::new()
        .faction(faction)
        .life_state(LifeState::Downed)
        .at(CellLevel::new(Cell::new(x, y), LEVEL))
        .spawn(app.world_mut())
}

/// Pushes a cycle [`ActIntent`] onto the shared seam (the keyboard / button write-point).
fn push(app: &mut App, intent: ActIntent) {
    app.world_mut()
        .resource_mut::<PendingActIntent>()
        .push(intent);
}

/// The current `SelectedShooter` entity, if any.
fn selection(app: &App) -> Option<Entity> {
    **app.world().resource::<SelectedShooter>()
}

/// C2 — `SelectNext` advances through the player gang in `(z, y, x)` order and WRAPS.
///
/// Three player gangers placed so their cell order is first, second, third. Starting selected
/// on the first, repeated `SelectNext` steps first → second → third → first (wrap).
#[test]
fn select_next_advances_and_wraps_in_cell_order() {
    let mut app = cycle_app();
    // Ascending (z, y, x): g_a (0,0) < g_b (1,0) < g_c (0,1).
    let g_a = placed_ganger(&mut app, PLAYER_FACTION, 0, 0);
    let g_b = placed_ganger(&mut app, PLAYER_FACTION, 1, 0);
    let g_c = placed_ganger(&mut app, PLAYER_FACTION, 0, 1);
    // Start on the first (the auto-select would also pick it); assert the wrapping walk.
    app.world_mut().insert_resource(SelectedShooter::new(g_a));

    push(&mut app, ActIntent::SelectNext);
    app.update();
    assert_eq!(selection(&app), Some(g_b), "Next from g_a -> g_b");

    push(&mut app, ActIntent::SelectNext);
    app.update();
    assert_eq!(selection(&app), Some(g_c), "Next from g_b -> g_c");

    push(&mut app, ActIntent::SelectNext);
    app.update();
    assert_eq!(selection(&app), Some(g_a), "Next from g_c WRAPS to g_a");
}

/// C2 — `SelectPrev` advances backward through the player gang in `(z, y, x)` order and WRAPS.
#[test]
fn select_prev_advances_and_wraps_in_cell_order() {
    let mut app = cycle_app();
    let g_a = placed_ganger(&mut app, PLAYER_FACTION, 0, 0);
    let g_b = placed_ganger(&mut app, PLAYER_FACTION, 1, 0);
    let g_c = placed_ganger(&mut app, PLAYER_FACTION, 0, 1);
    app.world_mut().insert_resource(SelectedShooter::new(g_a));

    push(&mut app, ActIntent::SelectPrev);
    app.update();
    assert_eq!(selection(&app), Some(g_c), "Prev from g_a WRAPS to g_c");

    push(&mut app, ActIntent::SelectPrev);
    app.update();
    assert_eq!(selection(&app), Some(g_b), "Prev from g_c -> g_b");

    push(&mut app, ActIntent::SelectPrev);
    app.update();
    assert_eq!(selection(&app), Some(g_a), "Prev from g_b -> g_a");
}

/// C2 PIN — the live cycle steps the deterministic `(z, y, x)` `cell_order_key`, NOT spawn /
/// query-iteration order NOR the allocation-order `Entity` id.
///
/// This is the discriminating twin of GTW-255's `auto_selects_deterministic_first_player_ganger`:
/// the three orderings are deliberately made to DIVERGE so the asserted cycle step pins the cell
/// sort (`SelectionCycleReads::ordered_player_gangers` → `cell_order_key`). The cell-order winner
/// for the step (`mid`) is the FIRST-spawned (so it is first in query iteration), so:
///
/// - `(z, y, x)` cell ordering → `[lo (0,0), mid (1,1), hi (9,9)]`; from `lo`, `Next` steps to
///   `mid` (cell index 1). This is the asserted, compliant pick.
/// - A forbidden NO-sort (spawn / iteration order) → `[mid, lo, hi]`; from `lo` (iter index 1),
///   `Next` steps to `hi` (iter index 2) ≠ `mid` → the assert FAILS.
/// - A forbidden `Entity`-id sort → `Entity::cmp` is opaque / non-allocation-monotonic in Bevy
///   0.19 (the GTW-255 finding), so its order is not the cell order either; the `assert_ne`
///   below forbids the spawn-order pick `hi` outright.
///
/// So reverting the `cell_order_key` sort to no-sort or to an `Entity`-id sort breaks THIS test —
/// the verification.md Rule 2 discriminating pin the coinciding-order tests above lack.
#[test]
fn cycle_steps_cell_order_not_spawn_or_entity_id_order() {
    let mut app = cycle_app();
    // Spawn OUT of cell order so spawn / iteration / Entity-id order all diverge from cell order.
    // `mid` spawned FIRST (first in query iteration) but is the cell-order MIDDLE.
    let mid = placed_ganger(&mut app, PLAYER_FACTION, 1, 1); // key (0, 1, 1)
    let lo = placed_ganger(&mut app, PLAYER_FACTION, 0, 0); // key (0, 0, 0) — cell-order FIRST
    let hi = placed_ganger(&mut app, PLAYER_FACTION, 9, 9); // key (0, 9, 9) — cell-order LAST
    // Start on the cell-order FIRST; one `Next` must reach the cell-order SECOND (`mid`).
    app.world_mut().insert_resource(SelectedShooter::new(lo));

    push(&mut app, ActIntent::SelectNext);
    app.update();
    assert_eq!(
        selection(&app),
        Some(mid),
        "Next from the cell-order first (lo) steps the (z, y, x) cell order to mid — NOT a \
         spawn/iteration-order pick (which would skip to hi)",
    );
    assert_ne!(
        selection(&app),
        Some(hi),
        "a spawn/iteration-order or Entity-id step (which would land on hi) must NOT win — only \
         the deterministic cell order",
    );

    // A second step lands on the cell-order LAST (`hi`), then a third WRAPS back to `lo` —
    // confirming the whole walk is in cell order, not iteration order.
    push(&mut app, ActIntent::SelectNext);
    app.update();
    assert_eq!(
        selection(&app),
        Some(hi),
        "Next from mid steps to hi (cell-order last)"
    );

    push(&mut app, ActIntent::SelectNext);
    app.update();
    assert_eq!(
        selection(&app),
        Some(lo),
        "Next from hi WRAPS to lo (cell-order first) — the walk is the cell order",
    );
}

/// C2 — the cycle SKIPS enemy gangers entirely: only player-faction gangers are ever selected.
#[test]
fn cycle_ignores_enemy_gangers() {
    let mut app = cycle_app();
    // Two player gangers interleaved with enemies in cell order.
    let p_a = placed_ganger(&mut app, PLAYER_FACTION, 0, 0);
    let _enemy_low = placed_ganger(&mut app, ENEMY_FACTION, 1, 0);
    let p_b = placed_ganger(&mut app, PLAYER_FACTION, 2, 0);
    let _enemy_high = placed_ganger(&mut app, ENEMY_FACTION, 3, 0);
    app.world_mut().insert_resource(SelectedShooter::new(p_a));

    // Next steps only between the two PLAYER gangers, wrapping (never an enemy).
    push(&mut app, ActIntent::SelectNext);
    app.update();
    assert_eq!(selection(&app), Some(p_b), "Next skips the enemy to p_b");

    push(&mut app, ActIntent::SelectNext);
    app.update();
    assert_eq!(
        selection(&app),
        Some(p_a),
        "Next WRAPS p_b -> p_a (skips enemies)"
    );
}

/// GTW-729 — the cycle SKIPS a DOWNED ganger to the next ALIVE one: a Downed ganger cannot
/// act, so it never enters the ordered gang and Prev/Next step straight PAST it (never landing
/// on it). Two ALIVE player gangers with a DOWNED one interleaved in cell order between them.
///
/// Pin-discriminating (verification.md Rule 2): before the GTW-729 Alive skip,
/// `ordered_player_gangers` filtered on `faction == player` alone, so the Downed ganger (cell
/// order between the two Alive) WOULD be the next step from `p_a` — this assertion would FAIL.
/// With the `is_alive` skip the cycle steps over it to `p_b`.
#[test]
fn cycle_skips_downed_gangers() {
    let mut app = cycle_app();
    // Cell order: p_a (0,0) < downed (1,0) < p_b (2,0). The Downed ganger sits BETWEEN the two
    // Alive player gangers, so a step that failed to skip it would land on it.
    let p_a = placed_ganger(&mut app, PLAYER_FACTION, 0, 0);
    let downed = placed_downed_ganger(&mut app, PLAYER_FACTION, 1, 0);
    let p_b = placed_ganger(&mut app, PLAYER_FACTION, 2, 0);
    app.world_mut().insert_resource(SelectedShooter::new(p_a));

    push(&mut app, ActIntent::SelectNext);
    app.update();
    assert_eq!(
        selection(&app),
        Some(p_b),
        "Next SKIPS the Downed ganger (cell-order between p_a and p_b) straight to p_b",
    );
    assert_ne!(
        selection(&app),
        Some(downed),
        "the cycle must NEVER land on a Downed ganger",
    );

    // Next again WRAPS back to p_a — still never the Downed one.
    push(&mut app, ActIntent::SelectNext);
    app.update();
    assert_eq!(
        selection(&app),
        Some(p_a),
        "Next WRAPS p_b -> p_a, still skipping the Downed ganger",
    );
}

// Note: the FIRST-from-`None` selection path (Next → first, Prev → last) is exercised by the
// pure `cycle_player_selection` unit tests in `gdtf_battle_input::selection::order` — the
// battle-start `auto_select_first_player_ganger` fills an empty `SelectedShooter` on the same
// frame `.before` the drain, so a live headless app never reaches the drain with `None` once a
// player ganger exists. The empty-player-gang no-op (where auto-select also cannot fill) is the
// integration twin below.

/// An EMPTY player gang → the cycle is a no-op (selection stays `None`).
#[test]
fn empty_player_gang_cycle_is_noop() {
    let mut app = cycle_app();
    // Only an enemy exists — no player ganger to cycle to.
    let _enemy = placed_ganger(&mut app, ENEMY_FACTION, 0, 0);
    app.world_mut().insert_resource(SelectedShooter::cleared());

    push(&mut app, ActIntent::SelectNext);
    app.update();
    assert_eq!(
        selection(&app),
        None,
        "Next over an empty player gang is a no-op"
    );

    push(&mut app, ActIntent::SelectPrev);
    app.update();
    assert_eq!(
        selection(&app),
        None,
        "Prev over an empty player gang is a no-op"
    );
}

/// C3 — `Tab` pushes `SelectNext` and `Shift+Tab` pushes `SelectPrev` through the SAME intent
/// seam (`cycle_selection_keys` → the drain). Driven by the REAL keyboard surface (a `Keybinds`
/// resource + a synthesized `ButtonInput<KeyCode>` just-press), proving the key path end-to-end.
#[test]
fn tab_cycles_next_and_shift_tab_cycles_prev() {
    let mut app = cycle_app();
    app.world_mut().insert_resource(test_keybinds());
    let g_a = placed_ganger(&mut app, PLAYER_FACTION, 0, 0);
    let g_b = placed_ganger(&mut app, PLAYER_FACTION, 1, 0);
    // A third ganger so the cell-order walk is non-trivial (its entity is not asserted here).
    placed_ganger(&mut app, PLAYER_FACTION, 0, 1);
    app.world_mut().insert_resource(SelectedShooter::new(g_a));

    // Plain Tab -> SelectNext.
    press_key(&mut app, KeyCode::Tab);
    app.update();
    assert_eq!(selection(&app), Some(g_b), "Tab cycles to the NEXT ganger");
    clear_keys(&mut app);

    // Shift+Tab -> SelectPrev (back to g_a).
    press_key(&mut app, KeyCode::ShiftLeft);
    press_key(&mut app, KeyCode::Tab);
    app.update();
    assert_eq!(
        selection(&app),
        Some(g_a),
        "Shift+Tab cycles to the PREVIOUS ganger"
    );
    clear_keys(&mut app);
}
