//! GTW-225 (GTW-48 S8): headless integration tests for ganger selection, the
//! selection highlight, level cycling, and the shared act-intent seam — updated for the
//! GTW-238 PlayerFaction-gated unified left-click decision (`left_click_act` replaces
//! `select_on_click`).
//!
//! - AC1 proves `GdtfBattleInputPlugin` `init_resource`s `SelectedShooter` (present +
//!   `None` after one update) and the `PendingActIntent` queue.
//! - drives the REAL `left_click_act` system: a synthesized `ButtonInput<MouseButton>`
//!   press + a `HoveredCell` + an `OccupancyGrid` PLAYER-faction occupant selects that
//!   occupant; an empty cell clears to `None`. GTW-238 gates SELECT to the player
//!   faction (an enemy occupant is NOT selected — covered in `control.rs`).
//! - drives `update_selection_highlight`: the one `SelectionHighlight` sprite snaps to
//!   `cell_to_world(selected cell)` visible, and hides on clear.
//! - AC6/AC9 drive level cycling THROUGH the seam: pushing a level-up intent +
//!   update mutates `ActiveLevel`, saturating at `MAX_LEVELS - 1` and flooring at 0.
//! - AC6 (keyboard real path) drives the REAL `level_keys` / `select_clear_key`
//!   systems: with a `Keybinds` resource inserted, a synthesized
//!   `ButtonInput<KeyCode>` just-pressed of the BOUND level-up / clear key flows
//!   key-press -> intent push -> `dispatch_act_intents` drain -> `ActiveLevel` /
//!   `SelectedShooter`, end-to-end (reverting either keyboard system fails these).
//! - AC11 proves the input layer is inert WITHOUT `BattleInProgress`.
//!
//! Every `app.world_mut()` mutation is in a TEST BODY — the accepted headless idiom
//! (`bevy-traps.md` #7 carve-out (a)). No function here takes `&mut World`/`&World`.

use bevy::{camera::visibility::RenderLayers, input::ButtonInput, prelude::*};
use gdtf_battle_input::{
    ActIntent, BoundKey, GdtfBattleInputPlugin, Keybinds, PendingActIntent, SelectedShooter,
    SelectionHighlight,
};
use gdtf_battle_presenter::{ActiveLevel, CELL_PX, WORLD_RENDER_LAYER, cell_to_world};
use gdtf_battle_sim::{
    BattleInProgress, BattleSimPlugin, Cell, CellLevel, Faction, Level, MAX_LEVELS, OccupancyGrid,
    PlayerFaction, Position,
};

/// The faction the player controls in these tests (matches `PlayerFaction`).
const PLAYER_FACTION: Faction = Faction::new(0);
/// An ENEMY faction (distinct from [`PLAYER_FACTION`]).
const ENEMY_FACTION: Faction = Faction::new(1);

/// Mints a valid throwaway [`Entity`] id without a panic (`Entity` has no public
/// numeric constructor in 0.18.1; spawning into a scratch world yields a real id).
fn mint_entity() -> Entity {
    World::new().spawn_empty().id()
}

/// Builds a focused headless selection app: `MinimalPlugins` + the
/// `GdtfBattleInputPlugin`, the presenter-owned `ActiveLevel`, the `BattleInProgress`
/// gate, an empty `OccupancyGrid`, `CombatTuning`, an empty `ButtonInput<MouseButton>`,
/// and the `PlayerFaction` the GTW-238 click decision gates on. (The keybind table is
/// asset-loaded, so under `MinimalPlugins` no `Keybinds` resolves — the keyboard systems
/// simply do not run; the level/seam tests push intents directly.)
fn selection_app(active_level: Level) -> App {
    use gdtf_battle_sim::tuning::CombatTuning;
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(GdtfBattleInputPlugin);
    app.world_mut().insert_resource(ActiveLevel(active_level));
    app.world_mut().insert_resource(BattleInProgress);
    app.world_mut().insert_resource(OccupancyGrid::default());
    app.world_mut().insert_resource(CombatTuning::default());
    app.world_mut()
        .insert_resource(PlayerFaction::new(PLAYER_FACTION));
    app.world_mut()
        .insert_resource(ButtonInput::<MouseButton>::default());
    app
}

/// Spawns a PLAYER-faction occupant at `cell` (so the GTW-238 SELECT branch picks it)
/// and returns its entity. Without a real `Faction` matching `PlayerFaction` the unified
/// left-click decision would not select it.
fn place_player_ganger(app: &mut App, cell: CellLevel) -> Entity {
    let ganger = app.world_mut().spawn(PLAYER_FACTION).id();
    app.world_mut()
        .resource_mut::<OccupancyGrid>()
        .set_occupant(cell, Some(ganger));
    ganger
}

/// Builds a selection app that ALSO has the keyboard press surface live: the
/// asset-loaded [`Keybinds`] table is inserted DIRECTLY (the keybinds.rs docs'
/// sanctioned headless idiom — "a test that wants them inserts `Keybinds`
/// directly") and an empty [`ButtonInput<KeyCode>`] is seeded so `level_keys` /
/// `select_clear_key` (which `Res`-read that buffer) run instead of failing param
/// validation under `MinimalPlugins` (no `InputPlugin`).
fn keyboard_app(active_level: Level) -> App {
    let mut app = selection_app(active_level);
    app.world_mut().insert_resource(test_keybinds());
    app.world_mut()
        .insert_resource(ButtonInput::<KeyCode>::default());
    app
}

/// A fully-bound [`Keybinds`] table for the keyboard real-path tests — every act is
/// on a distinct [`BoundKey`]. Built in the test body (not parsed from the shipped
/// `.ron`) so the keyboard tests do not depend on the editable file's chosen keys.
const fn test_keybinds() -> Keybinds {
    Keybinds {
        select_clear: BoundKey::KeyEscape,
        level_up:     BoundKey::KeyPageUp,
        level_down:   BoundKey::KeyPageDown,
        stance_cycle: BoundKey::KeyC,
        aim_toggle:   BoundKey::KeyF,
        facing_cycle: BoundKey::KeyR,
    }
}

/// Presses (just-pressed edge) a key in the `ButtonInput<KeyCode>` buffer.
fn press_key(app: &mut App, key: KeyCode) {
    let mut keys = app
        .world_mut()
        .get_resource_or_insert_with(ButtonInput::<KeyCode>::default);
    keys.press(key);
}

/// Sets the `HoveredCell` to a given cell (or clears it).
fn set_hovered(app: &mut App, cell: Option<CellLevel>) {
    use gdtf_battle_input::HoveredCell;
    app.world_mut().insert_resource(HoveredCell(cell));
}

/// Presses (just-pressed edge) the left mouse button in the input resource.
fn press_left(app: &mut App) {
    let mut mouse = app
        .world_mut()
        .get_resource_or_insert_with(ButtonInput::<MouseButton>::default);
    mouse.press(MouseButton::Left);
}

/// Releases the button + clears the edges, so a later `press` is a fresh
/// just-pressed (a held button never re-fires `just_pressed`).
fn clear_mouse(app: &mut App) {
    let mut mouse = app
        .world_mut()
        .get_resource_or_insert_with(ButtonInput::<MouseButton>::default);
    mouse.release(MouseButton::Left);
    mouse.clear();
}

/// The current `SelectedShooter` value.
fn selected(app: &App) -> Option<Entity> {
    app.world()
        .get_resource::<SelectedShooter>()
        .and_then(|s| **s)
}

/// The current `ActiveLevel`.
fn active_level(app: &App) -> Option<Level> {
    app.world().get_resource::<ActiveLevel>().map(|l| **l)
}

/// The single selection-highlight sprite's translation + visibility, if it exists.
fn highlight_state(app: &mut App) -> Option<(Vec3, Visibility)> {
    let mut q = app
        .world_mut()
        .query_filtered::<(&Transform, &Visibility), With<SelectionHighlight>>();
    q.iter(app.world()).next().map(|(t, v)| (t.translation, *v))
}

/// Counts the selection-highlight sprites in the world.
fn highlight_count(app: &mut App) -> usize {
    let mut q = app
        .world_mut()
        .query_filtered::<Entity, With<SelectionHighlight>>();
    q.iter(app.world()).count()
}

// ---------------------------------------------------------------------------------
// AC1 — the plugin init_resources the selection substrate.
// ---------------------------------------------------------------------------------

/// AC1 — `GdtfBattleInputPlugin` `init_resource`s `SelectedShooter` (present + `None`)
/// and the `PendingActIntent` queue (present + empty) after one update.
#[test]
fn plugin_init_resources_the_selection_substrate() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(GdtfBattleInputPlugin);
    app.update();

    assert_eq!(
        selected(&app),
        None,
        "SelectedShooter must be init_resource-d and start None",
    );
    let pending = app.world().get_resource::<PendingActIntent>();
    assert!(
        pending.is_some_and(PendingActIntent::is_empty),
        "PendingActIntent must be init_resource-d and start empty",
    );
}

// ---------------------------------------------------------------------------------
// AC3/AC4 — left-click selects the occupant; empty clears; faction-agnostic.
// ---------------------------------------------------------------------------------

/// A left-click on a PLAYER-faction-OCCUPIED hovered cell selects that occupant, driving
/// the REAL GTW-238 `left_click_act` SELECT branch.
#[test]
fn left_click_on_occupied_cell_selects_the_occupant() {
    let level = Level::new(0);
    let mut app = selection_app(level);

    // Place a player-faction occupant at a known cell and hover it.
    let cell = CellLevel::new(Cell::new(5, 7), level);
    let ganger = place_player_ganger(&mut app, cell);
    set_hovered(&mut app, Some(cell));

    press_left(&mut app);
    app.update();

    assert_eq!(
        selected(&app),
        Some(ganger),
        "a left-click on a player-faction occupied hovered cell must select its occupant",
    );
}

/// A left-click on an EMPTY hovered cell with a NON-player-faction selection clears the
/// selection to `None` (GTW-238 CLEAR branch — the selection is not a player ganger, so
/// neither FIRE nor MOVE applies and the chain falls through to CLEAR). The
/// player-selection + empty-cell → MOVE case is covered in `control.rs` (AC2).
#[test]
fn left_click_on_empty_cell_clears_the_selection() {
    let level = Level::new(0);
    let mut app = selection_app(level);

    // Pre-seed a NON-player selection (a bare entity with no Faction), then click an
    // empty (unoccupied) hovered cell. The selection is not a player ganger, so MOVE
    // does not apply and the chain clears.
    let ganger = mint_entity();
    app.world_mut()
        .insert_resource(SelectedShooter::new(ganger));
    set_hovered(&mut app, Some(CellLevel::new(Cell::new(1, 1), level)));

    press_left(&mut app);
    app.update();

    assert_eq!(
        selected(&app),
        None,
        "a left-click on an empty cell with a non-player selection must clear",
    );
}

/// GTW-238 — selection is FACTION-GATED: a PLAYER-faction occupant selects, but an
/// ENEMY-faction occupant does NOT become a player-own selection (the FIRE/MOVE/CLEAR
/// chain handles it, never SELECT). Drives the REAL `left_click_act` for each.
#[test]
fn selection_is_player_faction_gated() {
    let level = Level::new(0);

    // A player-faction occupant selects.
    {
        let mut app = selection_app(level);
        let cell = CellLevel::new(Cell::new(2, 2), level);
        let ganger = place_player_ganger(&mut app, cell);
        set_hovered(&mut app, Some(cell));
        press_left(&mut app);
        app.update();
        assert_eq!(
            selected(&app),
            Some(ganger),
            "a player-faction occupant must be selectable",
        );
    }

    // An enemy-faction occupant is NOT selected (no fire mode forces fire; with no prior
    // selection the chain falls through to CLEAR — never selects the enemy as own).
    {
        let mut app = selection_app(level);
        let enemy = app.world_mut().spawn(ENEMY_FACTION).id();
        let cell = CellLevel::new(Cell::new(40, 40), level);
        app.world_mut()
            .resource_mut::<OccupancyGrid>()
            .set_occupant(cell, Some(enemy));
        set_hovered(&mut app, Some(cell));
        press_left(&mut app);
        app.update();
        assert_ne!(
            selected(&app),
            Some(enemy),
            "an enemy-faction occupant must never become a player-own selection",
        );
        assert_eq!(
            selected(&app),
            None,
            "an enemy occupant with no fire mode + no prior selection clears",
        );
    }
}

// ---------------------------------------------------------------------------------
// AC5 — the selection highlight snaps to the selected cell + hides on clear.
// ---------------------------------------------------------------------------------

/// Exactly ONE `SelectionHighlight` sprite is drawn at `cell_to_world(selected cell)`,
/// visible, on the world render layer at one-cell size; clearing the selection (via the
/// `SelectionClear` seam — a player selection no longer clears on an empty-cell click,
/// it MOVEs) hides it (still one entity).
#[test]
fn selection_highlight_snaps_to_cell_and_hides_on_clear() {
    let level = Level::new(0);
    let mut app = selection_app(level);

    let cell = Cell::new(8, 3);
    let ganger = place_player_ganger(&mut app, CellLevel::new(cell, level));
    set_hovered(&mut app, Some(CellLevel::new(cell, level)));
    press_left(&mut app);
    app.update();

    assert_eq!(
        selected(&app),
        Some(ganger),
        "the player-faction occupant must be selected before checking the highlight",
    );
    assert_eq!(
        highlight_count(&mut app),
        1,
        "exactly one selection-highlight sprite exists after a selection",
    );
    assert_eq!(
        highlight_state(&mut app),
        Some((cell_to_world(cell, level), Visibility::Visible)),
        "the selection highlight must be visible at cell_to_world(selected cell)",
    );
    assert!(
        highlight_on_world_layer_at_cell_size(&mut app),
        "the selection highlight must be CELL_PX-sized on the WORLD_RENDER_LAYER",
    );

    // Clear the selection through the seam and confirm the highlight hides. (A
    // player-faction selection + an empty-cell click MOVEs under GTW-238, so the clear
    // is driven by the SelectionClear intent, not an empty click.) The drain clears the
    // selection in `dispatch_act_intents`, which is unordered vs the highlight system, so
    // a SECOND update lets the highlight observe the cleared selection.
    clear_mouse(&mut app);
    push_intent(&mut app, ActIntent::SelectionClear);
    app.update();
    assert_eq!(
        selected(&app),
        None,
        "the SelectionClear intent cleared the selection"
    );
    app.update();
    assert_eq!(
        highlight_count(&mut app),
        1,
        "the highlight entity persists (hidden, not duplicated) on clear",
    );
    assert_eq!(
        highlight_state(&mut app).map(|(_, v)| v),
        Some(Visibility::Hidden),
        "the selection highlight must hide when nothing is selected",
    );
}

/// Whether the one selection-highlight sprite is `CELL_PX`-sized + on the world layer.
fn highlight_on_world_layer_at_cell_size(app: &mut App) -> bool {
    let mut q = app
        .world_mut()
        .query_filtered::<(&Sprite, &RenderLayers), With<SelectionHighlight>>();
    let world_layer = RenderLayers::layer(WORLD_RENDER_LAYER);
    q.iter(app.world()).all(|(sprite, layers)| {
        sprite.custom_size == Some(Vec2::splat(CELL_PX)) && layers.intersects(&world_layer)
    })
}

// ---------------------------------------------------------------------------------
// AC6/AC9 — level cycling THROUGH the shared intent seam.
// ---------------------------------------------------------------------------------

/// Pushes an intent onto the shared `PendingActIntent` queue from the test body —
/// the same `push` the keyboard / `gdtf_app` button surfaces call.
fn push_intent(app: &mut App, intent: ActIntent) {
    app.world_mut()
        .resource_mut::<PendingActIntent>()
        .push(intent);
}

/// AC9 + AC6 — writing a `LevelUp` intent and updating drains it through the ONE
/// `dispatch_act_intents` system and mutates `ActiveLevel` (the seam is real).
#[test]
fn level_up_intent_raises_active_level_through_the_seam() {
    let level = Level::new(0);
    let mut app = selection_app(level);

    push_intent(&mut app, ActIntent::LevelUp);
    app.update();

    assert_eq!(
        active_level(&app),
        Some(Level::new(1)),
        "a LevelUp intent must raise ActiveLevel by one storey through the drain",
    );
    // The queue is drained (empty) after dispatch.
    assert!(
        app.world()
            .get_resource::<PendingActIntent>()
            .is_some_and(PendingActIntent::is_empty),
        "the intent queue must be drained after dispatch",
    );
}

/// AC6 — level-up SATURATES at `MAX_LEVELS - 1` and level-down FLOORS at 0, through
/// the seam.
#[test]
fn level_cycling_saturates_and_floors() {
    // Up from the top storey stays at the top.
    {
        let top = Level::new(MAX_LEVELS - 1);
        let mut app = selection_app(top);
        push_intent(&mut app, ActIntent::LevelUp);
        app.update();
        assert_eq!(
            active_level(&app),
            Some(top),
            "LevelUp must saturate at MAX_LEVELS - 1",
        );
    }
    // Down from the ground floor stays at 0.
    {
        let ground = Level::new(0);
        let mut app = selection_app(ground);
        push_intent(&mut app, ActIntent::LevelDown);
        app.update();
        assert_eq!(
            active_level(&app),
            Some(ground),
            "LevelDown must floor at 0",
        );
    }
}

/// AC9 — a `SelectionClear` intent through the seam clears `SelectedShooter`.
#[test]
fn selection_clear_intent_clears_through_the_seam() {
    let level = Level::new(0);
    let mut app = selection_app(level);
    let ganger = mint_entity();
    app.world_mut()
        .insert_resource(SelectedShooter::new(ganger));

    push_intent(&mut app, ActIntent::SelectionClear);
    app.update();

    assert_eq!(
        selected(&app),
        None,
        "a SelectionClear intent must clear the selection through the drain",
    );
}

// ---------------------------------------------------------------------------------
// AC6 (keyboard real path) — a bound KEY press drives the keyboard systems
// end-to-end: level_keys / select_clear_key -> intent -> dispatch -> state.
// ---------------------------------------------------------------------------------

/// AC6 — pressing the BOUND level-up key drives the REAL `level_keys` system: the
/// just-pressed `KeyCode` -> `ActIntent::LevelUp` push -> `dispatch_act_intents`
/// drain -> `ActiveLevel` rises one storey. Reverting `level_keys` (so no intent is
/// pushed) fails this — the key-press->ActiveLevel path is exercised, not bypassed.
#[test]
fn bound_level_up_key_press_raises_active_level() {
    let level = Level::new(0);
    let mut app = keyboard_app(level);
    let binds = test_keybinds();

    press_key(&mut app, binds.level_up());
    app.update();

    assert_eq!(
        active_level(&app),
        Some(Level::new(1)),
        "a press of the bound level-up key must raise ActiveLevel through level_keys",
    );
    // The intent the keyboard system pushed was drained by dispatch this update.
    assert!(
        app.world()
            .get_resource::<PendingActIntent>()
            .is_some_and(PendingActIntent::is_empty),
        "the keyboard-pushed intent must be drained after dispatch",
    );
}

/// AC6 — pressing the BOUND level-DOWN key drives `level_keys` the other way:
/// from an upper storey it lowers `ActiveLevel` by one (proving the down branch of
/// the real keyboard system runs, not just the up one).
#[test]
fn bound_level_down_key_press_lowers_active_level() {
    let level = Level::new(3);
    let mut app = keyboard_app(level);
    let binds = test_keybinds();

    press_key(&mut app, binds.level_down());
    app.update();

    assert_eq!(
        active_level(&app),
        Some(Level::new(2)),
        "a press of the bound level-down key must lower ActiveLevel through level_keys",
    );
}

/// AC6 — pressing the BOUND clear key drives the REAL `select_clear_key` system:
/// the just-pressed `KeyCode` -> `ActIntent::SelectionClear` push -> drain ->
/// `SelectedShooter` cleared. Reverting `select_clear_key` fails this.
#[test]
fn bound_clear_key_press_clears_selection() {
    let level = Level::new(0);
    let mut app = keyboard_app(level);
    let binds = test_keybinds();

    // Pre-seed a selection, then press the bound clear key.
    let ganger = mint_entity();
    app.world_mut()
        .insert_resource(SelectedShooter::new(ganger));

    press_key(&mut app, binds.select_clear());
    app.update();

    assert_eq!(
        selected(&app),
        None,
        "a press of the bound clear key must clear the selection through select_clear_key",
    );
}

/// AC6 — an UNBOUND key press does nothing: pressing a key no act is bound to
/// leaves `ActiveLevel` and `SelectedShooter` untouched (the keyboard systems read
/// the bound key off `Keybinds`, never a hardcoded literal — a stray press is inert).
#[test]
fn unbound_key_press_is_inert() {
    let level = Level::new(2);
    let mut app = keyboard_app(level);
    let ganger = mint_entity();
    app.world_mut()
        .insert_resource(SelectedShooter::new(ganger));

    // `Space` is bound to no act in `test_keybinds`.
    press_key(&mut app, KeyCode::Space);
    app.update();

    assert_eq!(
        active_level(&app),
        Some(level),
        "an unbound key press must not change ActiveLevel",
    );
    assert_eq!(
        selected(&app),
        Some(ganger),
        "an unbound key press must not clear the selection",
    );
}

// ---------------------------------------------------------------------------------
// AC11 — inert pre-battle (no BattleInProgress).
// ---------------------------------------------------------------------------------

/// AC11 — WITHOUT `BattleInProgress`, the input layer mutates nothing across several
/// updates and never panics: a queued intent is NOT drained (`ActiveLevel` unchanged),
/// a click does NOT select, no highlight spawns.
#[test]
fn inert_without_battle_in_progress() {
    let level = Level::new(2);
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(GdtfBattleInputPlugin);
    // NOTE: no BattleInProgress inserted.
    app.world_mut().insert_resource(ActiveLevel(level));
    app.world_mut().insert_resource(OccupancyGrid::default());

    // Queue a level-up + a click on an occupant, then run several updates.
    let ganger = mint_entity();
    let cell = CellLevel::new(Cell::new(3, 3), level);
    app.world_mut()
        .resource_mut::<OccupancyGrid>()
        .set_occupant(cell, Some(ganger));
    set_hovered(&mut app, Some(cell));
    push_intent(&mut app, ActIntent::LevelUp);
    press_left(&mut app);

    for _ in 0..4 {
        app.update();
    }

    assert_eq!(
        active_level(&app),
        Some(level),
        "ActiveLevel must be UNCHANGED pre-battle (the drain did not run)",
    );
    assert_eq!(
        selected(&app),
        None,
        "no selection must happen pre-battle (left_click_act did not run)",
    );
    assert_eq!(
        highlight_count(&mut app),
        0,
        "no selection highlight must spawn pre-battle",
    );
}

// ---------------------------------------------------------------------------------
// GTW-255 — auto-select the deterministic first player ganger at battle start.
// ---------------------------------------------------------------------------------

/// Spawns a ganger with a `Faction` + a `Position` at `cell` (the components the GTW-255
/// `auto_select_first_player_ganger` query reads) and returns its entity. Distinct from
/// `place_player_ganger` (which seeds occupancy for the click path) — the auto-select
/// reasons off `Position`, not the occupancy grid.
fn spawn_ganger(app: &mut App, faction: Faction, cell: CellLevel) -> Entity {
    app.world_mut().spawn((faction, Position::new(cell))).id()
}

/// AC1 — with no selection + player gangers present, one update auto-selects the
/// deterministic player ganger (the lowest `(level, y, x)` `Position` cell), and it is a
/// PLAYER-faction entity, never the enemy. Two player gangers at distinct cells + one
/// enemy: the lower-ordered player cell wins.
///
/// PIN (cell-ordering vs Entity-id AND spawn/iteration order). The cell-winner `mid` is
/// deliberately the MIDDLE spawn, so it is NEITHER the first found by query iteration NOR
/// the `Entity`-`Ord` minimum:
///
/// - `(level, y, x)` cell ordering → picks `mid` (cell `(x=1, y=1)` → key `(0, 1, 1)`,
///   below `first`'s `(0, 3, 2)` and `last`'s `(0, 9, 9)`). y=1 < y=3 also pins
///   y-before-x (`Cell::new(x, y)`).
/// - A forbidden first-found `.next()` / spawn-order pick → `first` (the first-spawned,
///   first in iteration) ≠ `mid` → FAILS.
/// - A forbidden `min_by_key(entity)` (Entity-id order, explicitly forbidden by the
///   ticket) → the `Entity`-`Ord` minimum, which in Bevy 0.18 is NOT the lowest spawn
///   index (`Entity::cmp` is opaque, not allocation-monotonic — exactly why the contract
///   forbids it). Empirically the minimum here is `last`, ≠ `mid` → FAILS.
///
/// So only the `(level, y, x)` cell ordering passes; both forbidden interpretations fail.
#[test]
fn auto_selects_deterministic_first_player_ganger() {
    let level = Level::new(0);
    let mut app = selection_app(level);

    // First spawned (first in query iteration) — higher cell `(x=2, y=3)` → key `(0,3,2)`.
    let first = spawn_ganger(
        &mut app,
        PLAYER_FACTION,
        CellLevel::new(Cell::new(2, 3), level),
    );
    // An enemy at the globally-lowest cell — must NEVER be auto-selected.
    let enemy = spawn_ganger(
        &mut app,
        ENEMY_FACTION,
        CellLevel::new(Cell::new(0, 0), level),
    );
    // The deterministic cell-winner — lowest player `(level, y, x)` cell `(x=1, y=1)` →
    // key `(0,1,1)`. Spawned in the MIDDLE: not first-found, not the Entity-Ord min.
    let mid = spawn_ganger(
        &mut app,
        PLAYER_FACTION,
        CellLevel::new(Cell::new(1, 1), level),
    );
    // Last spawned — highest cell `(x=9, y=9)` → key `(0,9,9)`; in Bevy 0.18 this is the
    // `Entity`-`Ord` minimum, so it is what a forbidden `min_by_key(entity)` would pick.
    let _last = spawn_ganger(
        &mut app,
        PLAYER_FACTION,
        CellLevel::new(Cell::new(9, 9), level),
    );

    assert_eq!(
        selected(&app),
        None,
        "precondition: nothing selected at start"
    );

    app.update();

    assert_eq!(
        selected(&app),
        Some(mid),
        "the lowest-Position player ganger by (level, y, x) must be auto-selected — by \
         cell ordering, NOT Entity id (forbidden pick = `_last`) NOR spawn/iteration \
         order (forbidden pick = `first`)",
    );
    assert_ne!(
        selected(&app),
        Some(first),
        "a spawn/iteration-order pick (first-spawned) must NOT win — only cell ordering",
    );
    assert_ne!(
        selected(&app),
        Some(enemy),
        "the enemy ganger (even at the lowest cell) must NEVER be auto-selected",
    );
}

/// AC1 (ordering, level-major) — the `(level, y, x)` ordering is LEVEL-major: a player
/// ganger on a lower storey wins over one with a smaller `(y, x)` on a higher storey.
///
/// PIN (cell-ordering vs Entity-id AND spawn/iteration order). The lower-storey cell-
/// winner is spawned in the MIDDLE, so it is NEITHER the first found NOR the `Entity`-`Ord`
/// minimum:
///
/// - level-major cell ordering → `lower_storey` (storey 0 beats both storey-2 and
///   storey-3 despite its larger `(y, x)`).
/// - forbidden first-found / spawn order → `high_first` (storey 2, spawned first) → FAILS.
/// - forbidden `min_by_key(entity)` → the `Entity`-`Ord` minimum, which in Bevy 0.18 is
///   the last-spawned `high_last` (storey 3), not the cell-winner → FAILS.
#[test]
fn auto_select_orders_level_major() {
    let mut app = selection_app(Level::new(0));

    // Higher storey, smaller (y, x) — spawned FIRST / first in iteration, must LOSE.
    let _high_first = spawn_ganger(
        &mut app,
        PLAYER_FACTION,
        CellLevel::new(Cell::new(0, 0), Level::new(2)),
    );
    // Lower storey, larger (y, x) — spawned MIDDLE (neither first-found nor Entity-Ord
    // min), must WIN on the level-major ordering.
    let lower_storey = spawn_ganger(
        &mut app,
        PLAYER_FACTION,
        CellLevel::new(Cell::new(50, 50), Level::new(0)),
    );
    // Higher storey still — spawned LAST, so in Bevy 0.18 it is the `Entity`-`Ord` minimum
    // (what a forbidden `min_by_key(entity)` would pick), yet must LOSE on storey.
    let _high_last = spawn_ganger(
        &mut app,
        PLAYER_FACTION,
        CellLevel::new(Cell::new(0, 0), Level::new(3)),
    );

    app.update();

    assert_eq!(
        selected(&app),
        Some(lower_storey),
        "a lower storey wins the (level, y, x) ordering even with a larger (y, x) — by \
         cell ordering, NOT Entity id (forbidden pick = `_high_last`) NOR spawn/iteration \
         order (forbidden pick = `_high_first`)",
    );
}

/// AC2 — an existing selection is NOT overridden: with a player ganger pre-selected (and
/// other lower-ordered player gangers present), several updates leave it UNCHANGED (the
/// auto-select only fills an EMPTY selection).
#[test]
fn auto_select_does_not_override_existing_selection() {
    let level = Level::new(0);
    let mut app = selection_app(level);

    // A higher-ordered player ganger is pre-selected; a lower-ordered one also exists.
    let chosen = spawn_ganger(
        &mut app,
        PLAYER_FACTION,
        CellLevel::new(Cell::new(9, 9), level),
    );
    let _lower = spawn_ganger(
        &mut app,
        PLAYER_FACTION,
        CellLevel::new(Cell::new(0, 0), level),
    );
    app.world_mut()
        .insert_resource(SelectedShooter::new(chosen));

    for _ in 0..4 {
        app.update();
    }

    assert_eq!(
        selected(&app),
        Some(chosen),
        "an existing selection must NOT be overridden by the auto-select",
    );
}

/// AC3 — enemy-only roster → no selection: with only enemy-faction gangers (no player
/// faction fielded), the selection stays `None` (an enemy is never auto-selected).
#[test]
fn auto_select_enemy_only_stays_none() {
    let level = Level::new(0);
    let mut app = selection_app(level);

    spawn_ganger(
        &mut app,
        ENEMY_FACTION,
        CellLevel::new(Cell::new(0, 0), level),
    );
    spawn_ganger(
        &mut app,
        ENEMY_FACTION,
        CellLevel::new(Cell::new(1, 1), level),
    );

    for _ in 0..4 {
        app.update();
    }

    assert_eq!(
        selected(&app),
        None,
        "with no player-faction ganger present the selection must stay None",
    );
}

/// AC4 — inert outside a live battle: WITHOUT `BattleInProgress`, even with player
/// gangers present the auto-select does not run (the gate held), the selection stays
/// `None`, and there is no panic.
#[test]
fn auto_select_inert_without_battle_in_progress() {
    let level = Level::new(0);
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(GdtfBattleInputPlugin);
    // NOTE: no BattleInProgress inserted; PlayerFaction present so only the battle gate
    // is the witness under test.
    app.world_mut().insert_resource(ActiveLevel(level));
    app.world_mut()
        .insert_resource(PlayerFaction::new(PLAYER_FACTION));

    // A player ganger that WOULD be auto-selected if the system ran.
    spawn_ganger(
        &mut app,
        PLAYER_FACTION,
        CellLevel::new(Cell::new(0, 0), level),
    );

    for _ in 0..4 {
        app.update();
    }

    assert_eq!(
        selected(&app),
        None,
        "the auto-select must be inert (selection stays None) without BattleInProgress",
    );
}

// ---------------------------------------------------------------------------------
// GTW play-test wave 3 (A2 + B) — the REAL battle-setup flow: a `SetupBattleRequested`
// (NOT a hand-inserted BattleInProgress/PlayerFaction/ganger) auto-selects a player
// ganger, and the selection highlight lands on THAT ganger's cell (never an empty cell).
//
// These drive the FULL runtime — `BattleSimPlugin` (which registers
// `setup_battle_on_request`, the system whose ordering vs the input band was the A2 bug)
// AND `GdtfBattleInputPlugin` (which registers the GTW-255 auto-select) together — so the
// ordering edge `InputSystems::Gather.after(setup_battle_on_request)` (the A2 fix) is
// exercised. The pre-existing GTW-255 tests above seed BattleInProgress + PlayerFaction +
// gangers DIRECTLY, bypassing setup — which is exactly the gap that let A2 ship.
// ---------------------------------------------------------------------------------

use gdtf_battle_sim::{
    SetupBattleRequested,
    armor::{
        ArmorFloor, ArmorHardness, ArmorIntegrity, ArmorPiece, ArmorProtection, ArmorType,
        SourceArmor,
    },
    ganger::{
        Aiming, Direction, Facing, Hp, LifeState, Luck, Shooting, Stance, StanceKind, Toughness,
        Tu, Wounds,
    },
    rng::BattleSeed,
    situation::{GangerSpawn, Situation},
    tuning::CombatTuning,
    weapon::{
        Accuracy, BaseSpread, DamageType, FatalBias, FireMode, FireModeSpec, Kickback,
        MagazineSize, ModeConeMult, ModeKind, ModeShots, ModeTuPercent, Stable, WeaponDamage,
        WeaponName, WeaponPunch, WeaponRegistry, WeaponShred, WeaponSpec,
    },
};

/// The weapon KEY every fixture ganger references — present in [`real_flow_registry`] so
/// the real `setup_battle_on_request` arms each spawned ganger (GTW-257).
const REAL_FLOW_WEAPON_KEY: &str = "test-weapon";

/// An arbitrary [`WeaponSpec`] (mechanism only, not shipped magnitudes) for the one
/// [`REAL_FLOW_WEAPON_KEY`] the fixture gangers reference.
fn real_flow_weapon_spec() -> WeaponSpec {
    WeaponSpec {
        base_spread:   BaseSpread::new(0.25),
        accuracy:      Accuracy::new(1.0),
        kickback:      Kickback::new(0.4),
        fatal_bias:    FatalBias::new(7.0),
        damage:        WeaponDamage::new(12),
        punch:         WeaponPunch::new(5),
        shred:         WeaponShred::new(3),
        damage_type:   DamageType::Kinetic,
        magazine_size: MagazineSize::new(30),
        fire_mode:     FireMode::new(vec![FireModeSpec::new(
            ModeKind::Single,
            ModeConeMult::new(1.0),
            ModeTuPercent::new(0.5),
            ModeShots::new(1),
        )]),
        stable:        Stable::new(false),
    }
}

/// The test [`WeaponRegistry`] — the one [`REAL_FLOW_WEAPON_KEY`] weapon the fixture
/// gangers reference, standing in for the app's `Load`-built registry.
fn real_flow_registry() -> WeaponRegistry {
    WeaponRegistry::new([(
        WeaponName::new(REAL_FLOW_WEAPON_KEY.to_owned()),
        real_flow_weapon_spec(),
    )])
}

/// An arbitrary roster-armor record (mechanism only) so a fixture ganger carries a
/// faithful `SourceArmor` for the real setup to copy into a `WornArmor`.
const fn real_flow_armor(base: i32) -> SourceArmor {
    SourceArmor::uniform(ArmorPiece::new(
        ArmorFloor::new(base),
        ArmorProtection::new(base + 1),
        ArmorIntegrity::new(base + 2),
        ArmorHardness::new(base + 3),
        ArmorType::DEFAULT,
    ))
}

/// Build an authored [`GangerSpawn`] at `at` (faction `faction`) with arbitrary-but-valid
/// component values, referencing the one [`REAL_FLOW_WEAPON_KEY`].
fn real_flow_ganger(at: CellLevel, faction: u8) -> GangerSpawn {
    GangerSpawn {
        at,
        faction: Faction::new(faction),
        facing: Facing::new(Direction::East),
        stance: Stance::new(StanceKind::Crouching),
        aiming: Aiming::new(true),
        hp: Hp::new(40),
        wounds: Wounds::new(3),
        tu: Tu::new(60),
        life_state: LifeState::Alive,
        shooting: Shooting::new(f32::from(faction) + 2.0),
        toughness: Toughness::new(f32::from(faction) + 3.0),
        luck: Luck::new(f32::from(faction) + 1.0),
        armor: real_flow_armor(i32::from(faction) + 1),
        weapon: WeaponName::new(REAL_FLOW_WEAPON_KEY.to_owned()),
    }
}

/// A two-ganger fixture (link-free → validates trivially): a PLAYER ganger (faction 0,
/// the default `player_faction`) and an ENEMY ganger (faction 1). The auto-select must
/// pick the player ganger, never the enemy.
fn real_flow_situation() -> Situation {
    let level = Level::new(0);
    Situation {
        gangers: vec![
            real_flow_ganger(CellLevel::new(Cell::new(5, 6), level), 0),
            real_flow_ganger(CellLevel::new(Cell::new(7, 8), level), 1),
        ],
        ..Situation::new()
    }
}

/// Builds the REAL-FLOW app: `MinimalPlugins` + BOTH the input plugin AND the sim's
/// `BattleSimPlugin`, plus the persistent `Load` resources a real app has before a battle
/// (`CombatTuning` + the `WeaponRegistry` the setup arms gangers from) and the presenter's
/// `ActiveLevel`. It deliberately does NOT insert `BattleInProgress` / `PlayerFaction` /
/// `OccupancyGrid` / any ganger — `setup_battle_on_request` creates all of those from the
/// `SetupBattleRequested` the test sends, so this exercises the production setup path.
fn real_flow_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(GdtfBattleInputPlugin)
        .add_plugins(BattleSimPlugin);
    app.world_mut().insert_resource(ActiveLevel(Level::new(0)));
    app.world_mut().insert_resource(CombatTuning::default());
    app.world_mut().insert_resource(real_flow_registry());
    // An empty mouse buffer so the input band's click systems pass param validation.
    app.world_mut()
        .insert_resource(ButtonInput::<MouseButton>::default());
    app
}

/// Sends the real `SetupBattleRequested` (the same message `gdtf_app`'s Generation scene
/// emits) carrying `situation` and an arbitrary seed.
fn request_setup(app: &mut App, situation: Situation) {
    app.world_mut().write_message(SetupBattleRequested::new(
        situation,
        BattleSeed::new(0x5A1C),
    ));
}

/// The faction of the entity currently selected, if any (looked up via a world query).
fn selected_faction(app: &mut App) -> Option<Faction> {
    let entity = selected(app)?;
    let world = app.world_mut();
    let mut query = world.query::<&Faction>();
    query.get(world, entity).ok().copied()
}

/// A2 (the mandatory regression test) — driving the REAL battle-setup flow auto-selects a
/// PLAYER-faction ganger ON THE SAME UPDATE the setup pours the battle in. A
/// `SetupBattleRequested` (NOT a hand-inserted BattleInProgress/PlayerFaction/ganger)
/// pours the battle in via `setup_battle_on_request`; after EXACTLY ONE `app.update()`
/// `SelectedShooter` is `Some` and the selected entity is a PLAYER-faction ganger.
///
/// PIN (the A2 ordering fix, the ONE-FRAME race the bug was):
/// `setup_battle_on_request` and the GTW-255 auto-select were both merely
/// `.before(SimSystems::Simulate)` with NO order between them. Without the A2 edge
/// (`InputSystems::Gather.after(setup_battle_on_request)`) there is no apply-deferred sync
/// point between setup's `Commands` (the `PlayerFaction` insert + the ganger spawns) and
/// the input band, so on the setup update the auto-select's
/// `run_if(resource_exists::<PlayerFaction>)` is FALSE (the insert is not applied yet) —
/// nothing is selected that frame (the in-engine "No ganger selected"). WITH the edge the
/// sync point applies setup's commands before the input band runs, so the selection lands
/// the SAME update. Asserting after EXACTLY ONE update discriminates: it FAILS without the
/// edge (selection still `None` after one update — it would not appear until update 2) and
/// PASSES with it.
///
/// This is also the gap the old GTW-255 tests missed: they seeded the gate + faction +
/// gangers directly, bypassing setup's ordering vs the input band. The test does NOT seed
/// `SelectedShooter`.
#[test]
fn real_setup_flow_auto_selects_a_player_ganger_same_update() {
    let mut app = real_flow_app();

    assert_eq!(
        selected(&app),
        None,
        "precondition: nothing selected before the battle is set up",
    );

    request_setup(&mut app, real_flow_situation());

    // EXACTLY ONE update: setup processes the message + (with the A2 edge) its Commands are
    // applied at the sync point BEFORE the input band's auto-select runs the same frame.
    app.update();

    // The setup actually ran this update (its gate witness + the player faction are present).
    assert!(
        app.world().get_resource::<BattleInProgress>().is_some(),
        "the real setup must have inserted BattleInProgress from the SetupBattleRequested",
    );
    assert_eq!(
        app.world().get_resource::<PlayerFaction>().map(|p| **p),
        Some(Faction::new(0)),
        "the real setup must have seeded PlayerFaction(0) from the situation default",
    );

    // A ganger IS auto-selected THIS update, and it is the PLAYER's (faction 0), never the
    // enemy. WITHOUT the A2 edge this is still `None` after one update (the regression).
    assert!(
        selected(&app).is_some(),
        "a player ganger must be auto-selected on the SAME update the real setup runs — NOT \
         left 'No ganger selected' (A2 regression: the missing setup->Gather sync point)",
    );
    assert_eq!(
        selected_faction(&mut app),
        Some(Faction::new(0)),
        "the auto-selected ganger must be a PLAYER-faction ganger, never the enemy",
    );
}

/// B — with a ganger auto-selected via the real flow, the ONE selection-highlight sprite
/// sits on the SELECTED GANGER's cell (its real `Position`/occupancy cell), and clicking
/// BARE FLOOR does NOT move the marker onto an empty cell.
///
/// The selection highlight scans the `OccupancyGrid` for the SELECTED entity's cell, so it
/// can only ever land on an occupied (the selected ganger's) cell. A bare-floor left-click
/// with a player selection is a MOVE under GTW-238 (no selection change), so the marker
/// stays on the ganger's cell — never the clicked empty cell.
#[test]
fn selection_highlight_tracks_selected_ganger_not_empty_cells() {
    let level = Level::new(0);
    let mut app = real_flow_app();
    request_setup(&mut app, real_flow_situation());
    for _ in 0..4 {
        app.update();
    }

    // A player ganger was auto-selected (A2); its cell is the player spawn (5, 6).
    let Some(selected_entity) = selected(&app) else {
        unreachable!("A2 must have auto-selected a player ganger before checking the highlight");
    };
    let ganger_cell = Cell::new(5, 6);

    // The ONE highlight sprite sits on the SELECTED ganger's cell, visible.
    assert_eq!(
        highlight_count(&mut app),
        1,
        "exactly one selection-highlight sprite exists once a ganger is selected",
    );
    assert_eq!(
        highlight_state(&mut app),
        Some((cell_to_world(ganger_cell, level), Visibility::Visible)),
        "the selection highlight must sit on the SELECTED ganger's cell (5, 6), not elsewhere",
    );

    // Click BARE FLOOR (an empty, in-bounds, unblocked cell). Under GTW-238 a player
    // selection + an empty cell is a MOVE, NOT a re-placement of the selection marker —
    // the marker must NOT jump to the clicked empty cell.
    let empty_cell = CellLevel::new(Cell::new(20, 20), level);
    set_hovered(&mut app, Some(empty_cell));
    press_left(&mut app);
    app.update();

    // The selection is unchanged (a MOVE does not touch SelectedShooter), so the marker
    // still tracks the selected ganger's occupancy cell — not the empty clicked cell.
    assert_eq!(
        selected(&app),
        Some(selected_entity),
        "a bare-floor click must not change the selection (GTW-238 MOVE, not re-select)",
    );
    let world_at_empty = cell_to_world(Cell::new(20, 20), level);
    let highlight = highlight_state(&mut app);
    assert!(
        highlight.is_some_and(|(t, _)| t != world_at_empty),
        "the selection marker must NOT move onto the clicked bare-floor cell (20, 20) — \
         it tracks the selected ganger, never an empty cell (B)",
    );
}
