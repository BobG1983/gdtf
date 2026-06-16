//! GTW-225 (GTW-48 S8): headless integration tests for ganger selection, the
//! selection highlight, level cycling, and the shared act-intent seam.
//!
//! - AC1 proves `GdtfBattleInputPlugin` `init_resource`s `SelectedShooter` (present +
//!   `None` after one update) and the `PendingActIntent` queue.
//! - AC3/AC4 drive the REAL `select_on_click` system: a synthesized
//!   `ButtonInput<MouseButton>` press + a `HoveredCell` + an `OccupancyGrid` occupant
//!   selects that occupant (faction-agnostic — both factions select), and an empty
//!   cell clears to `None`.
//! - AC5 drives `update_selection_highlight`: the one `SelectionHighlight` sprite
//!   snaps to `cell_to_world(selected cell)` visible, and hides on clear.
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
use gdtf_battle_sim::{BattleInProgress, Cell, CellLevel, Level, MAX_LEVELS, OccupancyGrid};

/// Mints a valid throwaway [`Entity`] id without a panic (`Entity` has no public
/// numeric constructor in 0.18.1; spawning into a scratch world yields a real id).
fn mint_entity() -> Entity {
    World::new().spawn_empty().id()
}

/// Builds a focused headless selection app: `MinimalPlugins` + the
/// `GdtfBattleInputPlugin`, the presenter-owned `ActiveLevel`, the `BattleInProgress`
/// gate, and an empty `OccupancyGrid`. (The keybind table is asset-loaded, so under
/// `MinimalPlugins` no `Keybinds` resolves — the keyboard systems simply do not run;
/// the level/seam tests push intents directly.)
fn selection_app(active_level: Level) -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(GdtfBattleInputPlugin);
    app.world_mut().insert_resource(ActiveLevel(active_level));
    app.world_mut().insert_resource(BattleInProgress);
    app.world_mut().insert_resource(OccupancyGrid::default());
    app
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
        select_clear:    BoundKey::KeyEscape,
        level_up:        BoundKey::KeyPageUp,
        level_down:      BoundKey::KeyPageDown,
        stance_cycle:    BoundKey::KeyC,
        aim_toggle:      BoundKey::KeyF,
        facing_cycle:    BoundKey::KeyR,
        fire_mode_cycle: BoundKey::KeyQ,
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

/// AC3 — a left-click on an OCCUPIED hovered cell selects that occupant, driving the
/// REAL `select_on_click` system.
#[test]
fn left_click_on_occupied_cell_selects_the_occupant() {
    let level = Level::new(0);
    let mut app = selection_app(level);

    // Place an occupant at a known cell and hover it.
    let ganger = mint_entity();
    let cell = CellLevel::new(Cell::new(5, 7), level);
    app.world_mut()
        .resource_mut::<OccupancyGrid>()
        .set_occupant(cell, Some(ganger));
    set_hovered(&mut app, Some(cell));

    press_left(&mut app);
    app.update();

    assert_eq!(
        selected(&app),
        Some(ganger),
        "a left-click on an occupied hovered cell must select its occupant",
    );
}

/// AC3 — a left-click on an EMPTY hovered cell clears the selection to `None`.
#[test]
fn left_click_on_empty_cell_clears_the_selection() {
    let level = Level::new(0);
    let mut app = selection_app(level);

    // Pre-seed a selection, then click an empty (unoccupied) hovered cell.
    let ganger = mint_entity();
    app.world_mut()
        .insert_resource(SelectedShooter::new(ganger));
    set_hovered(&mut app, Some(CellLevel::new(Cell::new(1, 1), level)));

    press_left(&mut app);
    app.update();

    assert_eq!(
        selected(&app),
        None,
        "a left-click on an empty hovered cell must clear the selection",
    );
}

/// AC4 — selection is FACTION-AGNOSTIC: an occupant placed on EITHER faction selects
/// identically (the system reads no `Faction`; the grid holds only the entity). Two
/// distinct entities standing in as "faction A" / "faction B" gangers both select.
#[test]
fn selection_is_faction_agnostic() {
    let level = Level::new(0);
    for cell_xy in [(2, 2), (40, 40)] {
        let mut app = selection_app(level);
        let ganger = mint_entity();
        let cell = CellLevel::new(Cell::new(cell_xy.0, cell_xy.1), level);
        app.world_mut()
            .resource_mut::<OccupancyGrid>()
            .set_occupant(cell, Some(ganger));
        set_hovered(&mut app, Some(cell));
        press_left(&mut app);
        app.update();
        assert_eq!(
            selected(&app),
            Some(ganger),
            "any occupant must be selectable regardless of faction (cell {cell_xy:?})",
        );
    }
}

// ---------------------------------------------------------------------------------
// AC5 — the selection highlight snaps to the selected cell + hides on clear.
// ---------------------------------------------------------------------------------

/// AC5 — exactly ONE `SelectionHighlight` sprite is drawn at `cell_to_world(selected
/// cell)`, visible, on the world render layer at one-cell size; clearing the
/// selection hides it (still one entity).
#[test]
fn selection_highlight_snaps_to_cell_and_hides_on_clear() {
    let level = Level::new(0);
    let mut app = selection_app(level);

    let ganger = mint_entity();
    let cell = Cell::new(8, 3);
    app.world_mut()
        .resource_mut::<OccupancyGrid>()
        .set_occupant(CellLevel::new(cell, level), Some(ganger));
    set_hovered(&mut app, Some(CellLevel::new(cell, level)));
    press_left(&mut app);
    app.update();

    assert_eq!(
        selected(&app),
        Some(ganger),
        "the occupant must be selected before checking the highlight",
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

    // Clear the selection (empty-cell click) and confirm the highlight hides.
    set_hovered(&mut app, Some(CellLevel::new(Cell::new(0, 0), level)));
    clear_mouse(&mut app);
    press_left(&mut app);
    app.update();
    assert_eq!(selected(&app), None, "the click cleared the selection");
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
        "no selection must happen pre-battle (select_on_click did not run)",
    );
    assert_eq!(
        highlight_count(&mut app),
        0,
        "no selection highlight must spawn pre-battle",
    );
}
