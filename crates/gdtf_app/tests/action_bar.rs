//! GTW-228 (GTW-48 S9 / 222c): headless integration tests for the themed UI
//! action-bar — the `gdtf_app`-side button surface over the SAME 222a act-intent seam
//! the keyboard surface writes.
//!
//! The bar lives in `gdtf_app`'s battlescape and is driven here through the REAL
//! stack: a `GdtfTestAppBuilder` headless walk to `BattleScapeState::BattleRunning`
//! (the GTW-221 / `battle_running_driver.rs` battlescape-walk precedent) spawns the
//! real action-bar (its `OnEnter(BattleRunning)` `spawn_action_bar` runs with the
//! injected `default_theme()`), and the tests synthesize `Interaction = Pressed`
//! (`Changed`) on a REAL spawned button — the exact swap a real mouse click drives via
//! `ui_focus_system` (`bevy-traps.md` #6) — then assert the act resolves identically to
//! the equivalent key/intent surface:
//!
//! - AC1 — the bar spawns N markered, interactive `Button`s in the live battle and is
//!   despawned outside it.
//! - AC3 — a stance / aim button press writes the SAME `*Requested` the equivalent
//!   222a intent does, byte-for-byte (the `acts.rs` AC5 parity idiom); the level
//!   buttons mutate `ActiveLevel` like the level intent.
//! - AC4 — the fire-mode-select button advances `SelectedFireMode` only among the
//!   selected weapon's offered modes, reusing 222b's cycle helper.
//! - AC5 — the DEFERRED reload / end-turn buttons carry `DisabledButton` and emit NO
//!   intent under a synthesized press.
//! - AC6 — with NO `SelectedShooter`, an act-button press is a no-op (no message, no
//!   panic).
//! - AC7 — each button is a `bevy_ui` `Button` + `Node` tree, NOT a
//!   `WORLD_RENDER_LAYER` sprite — it routes to the GTW-120 UI camera.
//!
//! Every `app.world_mut()` mutation is in a TEST BODY — the accepted headless idiom
//! (`bevy-traps.md` #7 carve-out (a)). No function here takes `&mut World`/`&World`.
//! Reaching a live battle + driving the GUI itself is the post-gate QA carve-out
//! (AC9, inEngineEvidence); these synthesized-`Interaction` parity tests are the
//! strong evidence.

use bevy::{
    camera::visibility::RenderLayers,
    ecs::entity::Entity,
    prelude::*,
    state::state::State,
    ui::{Interaction, Node, widget::Button},
};
use gdtf_app::test_support::{
    AimToggleButton, AppState, BattleRunningComplete, BattleScapeState, EndTurnButton,
    FireModeSelectButton, FleeButton, LevelDownButton, LevelUpButton, ReloadButton, RunningState,
    StanceCycleButton,
};
use gdtf_battle_input::{
    ActIntent, PendingActIntent, SelectedFireMode, SelectedShooter, next_stance,
};
use gdtf_battle_presenter::{ActiveLevel, WORLD_RENDER_LAYER};
use gdtf_battle_sim::{
    Aiming, BattleInProgress, Direction, Facing, FireMode, FireModeSpec, Level, ModeConeMult,
    ModeShots, ModeTuPercent, Stance, StanceKind,
    acts::{SetAimingRequested, SetStanceRequested},
    tuning::CombatTuning,
};
use gdtf_test_utils::{GdtfTestAppBuilder, advance_until};
use gdtf_ui::{DisabledButton, theme::default_theme};

/// A budget large enough to drive the deep walk into the battlescape (each leaf scene
/// spends a couple of `FixedUpdate` ticks plus transition propagation), bounded so a
/// machine that never reaches the predicate fails instead of hanging (the
/// `battle_running_driver.rs` budget).
const BUDGET: u32 = 96;

/// The number of EXISTING-act buttons the bar spawns (stance, aim, fire-mode-select,
/// level-up, level-down). The two DEFERRED buttons (reload, end-turn) are counted
/// separately where relevant.
const EXISTING_ACT_BUTTONS: usize = 5;

// ---------------------------------------------------------------------------------
// Harness — drive the real stack to BattleRunning, where the bar is live.
// ---------------------------------------------------------------------------------

/// Reads the current [`BattleScapeState`] if it is active.
fn battlescape_state(app: &App) -> Option<BattleScapeState> {
    app.world()
        .get_resource::<State<BattleScapeState>>()
        .map(|state| *state.get())
}

/// Reads the current [`RunningState`] if it is active.
fn running_state(app: &App) -> Option<RunningState> {
    app.world()
        .get_resource::<State<RunningState>>()
        .map(|state| *state.get())
}

/// Builds the headless walk app, injecting the persistent `Load` resources the machine
/// needs to traverse `Load` (no `AssetServer` under `MinimalPlugins`) — `default_theme()`
/// (which `spawn_action_bar` reads) + `CombatTuning`. No `LoadedSituation` → the empty
/// `Situation::default()` battle is set up, which still makes `BattleInProgress` present
/// in `BattleRunning` (the bar's action-system gate).
fn walk_app() -> App {
    let mut app = GdtfTestAppBuilder::new()
        .starting_in(AppState::Running)
        .build();
    app.world_mut().insert_resource(default_theme());
    app.world_mut().insert_resource(CombatTuning::default());
    app
}

/// Drives the app from `Running`/`Menu` down to the first update on which
/// [`BattleScapeState::BattleRunning`] is active (the player at the menu picks the
/// Battlescape transition). Returns whether it was reached.
fn drive_to_battle_running(app: &mut App) -> bool {
    // Stand in for the player at the (no-auto-advance) menu: wait for Menu to rest, then
    // queue Menu -> Game (the Battlescape button's transition).
    let at_menu = advance_until(
        app,
        |app| running_state(app) == Some(RunningState::Menu),
        BUDGET,
    );
    if !at_menu {
        return false;
    }
    app.world_mut()
        .resource_mut::<NextState<RunningState>>()
        .set(RunningState::Game);
    advance_until(
        app,
        |app| battlescape_state(app) == Some(BattleScapeState::BattleRunning),
        BUDGET,
    )
}

/// Drives the walk to `BattleRunning` and returns the app, asserting the descent
/// succeeded (so each test starts from the live battle where the bar is spawned).
fn battle_running_app() -> App {
    let mut app = walk_app();
    assert!(
        drive_to_battle_running(&mut app),
        "the walk should reach BattleScapeState::BattleRunning within {BUDGET} updates; last \
         observed BattleScapeState was {:?}",
        battlescape_state(&app),
    );
    app
}

/// Looks up the single entity carrying marker `M`, if exactly one exists (the
/// `menu_actions.rs` `single_with` idiom).
fn single_with<M: Component>(app: &mut App) -> Option<Entity> {
    let mut q = app.world_mut().query_filtered::<Entity, With<M>>();
    let found: Vec<Entity> = q.iter(app.world()).collect();
    match found.as_slice() {
        [one] => Some(*one),
        _ => None,
    }
}

/// Asserts exactly one button carrying marker `M` exists and returns it, so callers can
/// `let Some(b) = require_button::<M>(..) else { return };` without a denied
/// `assert!(false)` guard (the `assert!(cond); let else { return }` idiom).
fn require_button<M: Component>(app: &mut App) -> Option<Entity> {
    let found = single_with::<M>(app);
    assert!(
        found.is_some(),
        "exactly one button of the expected marker must be spawned in BattleRunning",
    );
    found
}

/// Synthesizes a fresh mouse press on `button` by setting its [`Interaction`] to
/// [`Pressed`] (the swap `ui_focus_system` drives for a real click). A direct write to
/// the component marks it `Changed` this update, so the `Changed<Interaction>` action
/// query fires.
fn press_button(app: &mut App, button: Entity) {
    if let Some(mut interaction) = app.world_mut().get_mut::<Interaction>(button) {
        *interaction = Interaction::Pressed;
    }
}

/// One fire-mode spec with a marker `tu_percent` so the rungs are distinct authored
/// values (magnitudes arbitrary, not pinned tuning — the `acts.rs` precedent).
const fn spec(tu_percent: f32, shots: u16) -> FireModeSpec {
    FireModeSpec::new(
        ModeConeMult::new(1.0),
        ModeTuPercent::new(tu_percent),
        ModeShots::new(shots),
    )
}

/// A `SingleBurstFullAuto` selector with three distinct rungs.
const fn sbf_selector() -> FireMode {
    FireMode::SingleBurstFullAuto {
        single:    spec(0.2, 1),
        burst:     spec(0.4, 3),
        full_auto: spec(0.7, 6),
    }
}

// ---------------------------------------------------------------------------------
// Message probes — collect the *Requested emitted this run into resources read in the
// test body (each probe runs after the drain, so it sees the same update's emission).
// ---------------------------------------------------------------------------------

/// Collected `SetStanceRequested` messages (probe).
#[derive(Resource, Default)]
struct StanceProbe(Vec<SetStanceRequested>);
/// Collected `SetAimingRequested` messages (probe).
#[derive(Resource, Default)]
struct AimProbe(Vec<SetAimingRequested>);

/// Adds the two posture-message probes, each running AFTER the intent drain so it
/// observes the same update's emitted messages (the `acts.rs` `add_probes` idiom). The
/// probes have their own `MessageReader` cursors (independent of the sim's `dispatch_*`),
/// so they read every message the drain wrote.
fn add_probes(app: &mut App) {
    app.world_mut().insert_resource(StanceProbe::default());
    app.world_mut().insert_resource(AimProbe::default());
    app.add_systems(
        Update,
        (
            |mut r: MessageReader<SetStanceRequested>, mut p: ResMut<StanceProbe>| {
                p.0.extend(r.read().copied());
            },
            |mut r: MessageReader<SetAimingRequested>, mut p: ResMut<AimProbe>| {
                p.0.extend(r.read().copied());
            },
        )
            .after(gdtf_battle_input::dispatch_act_intents),
    );
}

/// The collected `SetStanceRequested` messages.
fn stances(app: &App) -> Vec<SetStanceRequested> {
    app.world()
        .get_resource::<StanceProbe>()
        .map(|p| p.0.clone())
        .unwrap_or_default()
}

/// The collected `SetAimingRequested` messages.
fn aims(app: &App) -> Vec<SetAimingRequested> {
    app.world()
        .get_resource::<AimProbe>()
        .map(|p| p.0.clone())
        .unwrap_or_default()
}

/// Spawns a ganger carrying exactly the components the act systems read
/// (`Stance`/`Facing`/`Aiming`/`FireMode`) and SELECTS it via the `SelectedShooter`
/// resource — the selection the act drain reads. Returns its entity. (The act surface
/// only reads `*SelectedShooter`, so setting the resource directly is the faithful,
/// minimal selection for these button tests; the cursor-click selection path is covered
/// in `gdtf_battle_input`'s `acts.rs`.)
fn arm_and_select(
    app: &mut App,
    selector: FireMode,
    stance: StanceKind,
    facing: Direction,
) -> Entity {
    let ganger = app
        .world_mut()
        .spawn((
            Stance::new(stance),
            Facing::new(facing),
            Aiming::new(false),
            selector,
        ))
        .id();
    app.world_mut()
        .insert_resource(SelectedShooter::new(ganger));
    ganger
}

/// The current `ActiveLevel` storey as a plain `u8`, if present.
fn active_level(app: &App) -> Option<u8> {
    app.world().get_resource::<ActiveLevel>().map(|l| *(**l))
}

/// The current `SelectedFireMode` spec, if present.
fn selected_mode(app: &App) -> Option<FireModeSpec> {
    app.world().get_resource::<SelectedFireMode>().map(|m| **m)
}

// ---------------------------------------------------------------------------------
// AC1 — the bar spawns N markered, interactive buttons in BattleRunning and is
// despawned / inert outside it.
// ---------------------------------------------------------------------------------

/// AC1 — in the live battle the action-bar has spawned one interactive `Button` per
/// EXISTING act (each carrying `Button` + `Interaction` + its action marker), and once
/// the battle leaves `BattleRunning` the buttons are despawned.
#[test]
fn action_bar_spawns_in_battle_and_despawns_outside() {
    let mut app = battle_running_app();

    // Each existing-act marker resolves to exactly one entity carrying Button +
    // Interaction (interactive).
    let buttons = [
        require_button::<StanceCycleButton>(&mut app),
        require_button::<AimToggleButton>(&mut app),
        require_button::<FireModeSelectButton>(&mut app),
        require_button::<LevelUpButton>(&mut app),
        require_button::<LevelDownButton>(&mut app),
    ];
    let present = buttons.iter().filter(|b| b.is_some()).count();
    assert_eq!(
        present, EXISTING_ACT_BUTTONS,
        "the bar spawns exactly one button per existing act",
    );
    for found in buttons {
        let Some(button) = found else { return };
        assert!(
            app.world().get::<Button>(button).is_some(),
            "an action button must carry Button",
        );
        assert!(
            app.world().get::<Interaction>(button).is_some(),
            "an action button must carry Interaction (interactive)",
        );
    }

    // Leave BattleRunning: the battlescape now PERSISTS (GTW-236, the placeholder budget
    // auto-exit is gone), so the test inserts the explicit `BattleRunningComplete`
    // end-signal marker (standing in for the not-yet-wired victory/flee) to trip `move_on`
    // and advance the machine out of BattleRunning, where `OnExit` despawns the bar.
    app.world_mut().insert_resource(BattleRunningComplete);
    let left_battle_running = advance_until(
        &mut app,
        |app| battlescape_state(app) != Some(BattleScapeState::BattleRunning),
        BUDGET,
    );
    assert!(
        left_battle_running,
        "an explicit BattleRunningComplete insert must advance the machine out of BattleRunning \
         within {BUDGET} updates",
    );
    assert!(
        single_with::<StanceCycleButton>(&mut app).is_none(),
        "the action bar must be despawned once the battle leaves BattleRunning",
    );
}

// ---------------------------------------------------------------------------------
// AC3 — a button press writes the SAME *Requested the equivalent intent does,
// byte-for-byte (over the REAL 222a drain).
// ---------------------------------------------------------------------------------

/// AC3 — pressing the stance button emits one `SetStanceRequested` for `*SelectedShooter`
/// with the next-of-cycle stance, byte-for-byte EQUAL to the message the direct
/// `StanceCycle` intent (the key surrogate) produces over the SAME seam.
#[test]
fn stance_button_emits_next_of_cycle_and_matches_direct_intent() {
    // Via the BUTTON press.
    let mut app = battle_running_app();
    add_probes(&mut app);
    let ganger = arm_and_select(
        &mut app,
        sbf_selector(),
        StanceKind::Standing,
        Direction::North,
    );
    let Some(button) = require_button::<StanceCycleButton>(&mut app) else {
        return;
    };
    press_button(&mut app, button);
    app.update();

    let via_button = stances(&app);
    assert_eq!(
        via_button.len(),
        1,
        "one SetStanceRequested via the stance button"
    );
    assert_eq!(via_button[0].actor, ganger, "actor = *SelectedShooter");
    assert_eq!(
        via_button[0].stance,
        next_stance(StanceKind::Standing),
        "the next-of-cycle stance",
    );

    // Via the direct intent (the key surrogate) over the SAME seam.
    let mut app2 = battle_running_app();
    add_probes(&mut app2);
    let _ganger2 = arm_and_select(
        &mut app2,
        sbf_selector(),
        StanceKind::Standing,
        Direction::North,
    );
    app2.world_mut()
        .resource_mut::<PendingActIntent>()
        .push(ActIntent::StanceCycle);
    app2.update();
    let via_intent = stances(&app2);
    assert_eq!(via_intent.len(), 1, "one SetStanceRequested via the intent");

    assert_eq!(
        via_button[0], via_intent[0],
        "the button and the intent (key) surface must produce byte-for-byte equal \
         SetStanceRequested",
    );
}

/// AC3 — pressing the aim button emits one `SetAimingRequested` toggling the actor's aim,
/// byte-for-byte EQUAL to the direct `AimToggle` intent's message.
#[test]
fn aim_button_toggles_and_matches_direct_intent() {
    let mut app = battle_running_app();
    add_probes(&mut app);
    let ganger = arm_and_select(
        &mut app,
        sbf_selector(),
        StanceKind::Standing,
        Direction::North,
    );
    let Some(button) = require_button::<AimToggleButton>(&mut app) else {
        return;
    };
    press_button(&mut app, button);
    app.update();

    let via_button = aims(&app);
    assert_eq!(
        via_button.len(),
        1,
        "one SetAimingRequested via the aim button"
    );
    assert_eq!(via_button[0].actor, ganger, "actor = *SelectedShooter");

    let mut app2 = battle_running_app();
    add_probes(&mut app2);
    let _ganger2 = arm_and_select(
        &mut app2,
        sbf_selector(),
        StanceKind::Standing,
        Direction::North,
    );
    app2.world_mut()
        .resource_mut::<PendingActIntent>()
        .push(ActIntent::AimToggle);
    app2.update();
    let via_intent = aims(&app2);
    assert_eq!(via_intent.len(), 1, "one SetAimingRequested via the intent");

    assert_eq!(
        via_button[0], via_intent[0],
        "the button and the intent (key) surface must produce byte-for-byte equal \
         SetAimingRequested",
    );
}

/// AC3 — pressing the level-up button raises `ActiveLevel` by one storey (the same
/// `ActiveLevel` mutation the `LevelUp` intent drives), and level-down lowers it.
#[test]
fn level_buttons_step_active_level_like_the_intent() {
    let mut app = battle_running_app();
    app.world_mut().insert_resource(ActiveLevel(Level::new(0)));

    let Some(up) = require_button::<LevelUpButton>(&mut app) else {
        return;
    };
    press_button(&mut app, up);
    app.update();
    assert_eq!(
        active_level(&app),
        Some(1),
        "the level-up button raises ActiveLevel by one storey",
    );

    let Some(down) = require_button::<LevelDownButton>(&mut app) else {
        return;
    };
    press_button(&mut app, down);
    app.update();
    assert_eq!(
        active_level(&app),
        Some(0),
        "the level-down button lowers ActiveLevel by one storey",
    );
}

// ---------------------------------------------------------------------------------
// AC4 — the fire-mode-select button advances SelectedFireMode only among the selected
// weapon's offered modes, reusing 222b's cycle helper.
// ---------------------------------------------------------------------------------

/// AC4 — a `Single`-armed shooter's fire-mode button press never reaches `burst` (stays
/// on `single`); a `SingleBurstFullAuto`-armed shooter's repeated presses advance
/// single -> burst -> `full_auto` and wrap — asserted against the authored 222a cyclic
/// order (the 222b `next_fire_mode` helper drives the drain, not an inlined literal).
#[test]
fn fire_mode_button_walks_only_offered_modes() {
    let single = spec(0.2, 1);
    let burst = spec(0.4, 3);
    let full_auto = spec(0.7, 6);
    let sbf = FireMode::SingleBurstFullAuto {
        single,
        burst,
        full_auto,
    };

    // SingleBurstFullAuto: each single button-press steps ONE rung of the offered ladder,
    // wrapping full_auto -> single — each step exercised in a fresh battle so the press is
    // the single post-arm press the harness drives cleanly (the AC3 working recipe; the
    // ladder + wrap are the real 222b `next_fire_mode` drain).
    assert_eq!(
        press_fire_mode_from(sbf, single),
        Some(burst),
        "single must step to burst",
    );
    assert_eq!(
        press_fire_mode_from(sbf, burst),
        Some(full_auto),
        "burst must step to full_auto",
    );
    assert_eq!(
        press_fire_mode_from(sbf, full_auto),
        Some(single),
        "full_auto must wrap back to single",
    );

    // Single: a Single-armed shooter's button press never reaches burst — it stays on
    // single (no other mode is offered / invented).
    let single_only = FireMode::Single { single };
    assert_eq!(
        press_fire_mode_from(single_only, single),
        Some(single),
        "a Single weapon's fire-mode button stays on single",
    );
}

/// Builds a fresh battle, arms+selects a ganger with `selector`, sets the starting fire
/// mode to `start`, presses the fire-mode button ONCE (the AC3 single-press recipe), and
/// returns the resulting `SelectedFireMode` spec.
///
/// The starting mode is set AFTER the real selection-default has settled (one update, so
/// `sync_fire_mode_on_select`'s reset-to-single fires and the `SelectedShooter` change
/// flag clears), so the single press then steps the REAL 222b cycle from `start` without
/// the default-on-select clobbering it.
fn press_fire_mode_from(selector: FireMode, start: FireModeSpec) -> Option<FireModeSpec> {
    let mut app = battle_running_app();
    arm_and_select(&mut app, selector, StanceKind::Standing, Direction::North);
    // Settle the selection default + clear the SelectedShooter change flag.
    app.update();
    // Pin the starting rung (after the default-on-select has run).
    app.world_mut()
        .insert_resource(SelectedFireMode::new(start));
    let button = single_with::<FireModeSelectButton>(&mut app);
    assert!(button.is_some(), "the fire-mode button must be spawned");
    let button = button?;
    press_button(&mut app, button);
    app.update();
    selected_mode(&app)
}

// ---------------------------------------------------------------------------------
// AC5 — a DEFERRED button (reload / end-turn) is DisabledButton and emits NO intent
// under any interaction.
// ---------------------------------------------------------------------------------

/// AC5 — the reload + end-turn buttons are rendered as `DisabledButton` and a
/// synthesized press emits ZERO messages / intents on the seam (the
/// `Without<DisabledButton>` action filter excludes them — the menu precedent).
#[test]
fn deferred_buttons_are_disabled_and_emit_nothing() {
    let mut app = battle_running_app();
    add_probes(&mut app);
    // Arm + select a ganger so an act WOULD emit if a deferred button leaked an intent.
    arm_and_select(
        &mut app,
        sbf_selector(),
        StanceKind::Standing,
        Direction::North,
    );

    let deferred = [
        require_button::<ReloadButton>(&mut app),
        require_button::<EndTurnButton>(&mut app),
    ];
    for found in deferred {
        let Some(button) = found else { return };
        assert!(
            app.world().get::<DisabledButton>(button).is_some(),
            "a deferred button must carry DisabledButton",
        );
        press_button(&mut app, button);
    }
    app.update();

    assert!(
        stances(&app).is_empty() && aims(&app).is_empty(),
        "a DisabledButton press must emit NO *Requested on the seam",
    );
    assert!(
        app.world()
            .get_resource::<PendingActIntent>()
            .is_some_and(PendingActIntent::is_empty),
        "a DisabledButton press must queue NO intent",
    );
}

// ---------------------------------------------------------------------------------
// AC6 — with NO SelectedShooter, an act-button press is a no-op (no message, no panic).
// ---------------------------------------------------------------------------------

/// AC6 — with the selection cleared, pressing each act button across several updates
/// emits ZERO `*Requested` and does not panic (the drain finds nothing to act on).
#[test]
fn no_selection_makes_act_buttons_a_no_op() {
    let mut app = battle_running_app();
    add_probes(&mut app);
    app.world_mut().insert_resource(SelectedShooter::cleared());

    // Press the stance + aim buttons (the *Requested-emitting acts) with no selection.
    let act_buttons = [
        single_with::<StanceCycleButton>(&mut app),
        single_with::<AimToggleButton>(&mut app),
    ];
    for button in act_buttons.into_iter().flatten() {
        press_button(&mut app, button);
    }
    // Several updates to prove no deferred panic / late emission.
    for _ in 0..3 {
        app.update();
    }

    assert!(
        stances(&app).is_empty(),
        "no SetStanceRequested without a selection"
    );
    assert!(
        aims(&app).is_empty(),
        "no SetAimingRequested without a selection"
    );
}

// ---------------------------------------------------------------------------------
// AC7 — the bar renders on the UI camera (a bevy_ui Button/Node tree), NOT a
// WORLD_RENDER_LAYER sprite.
// ---------------------------------------------------------------------------------

/// AC7 — each action button carries `Button` + `Node` and does NOT carry
/// `RenderLayers::layer(WORLD_RENDER_LAYER)` (`bevy_ui` routes it to the highest-order =
/// UI camera per `bevy-traps.md` #6 — no extra `Pickable` / `IsDefaultUiCamera` / picking
/// plugin).
#[test]
fn buttons_are_ui_nodes_not_world_render_layer_sprites() {
    let mut app = battle_running_app();
    let world_layer = RenderLayers::layer(WORLD_RENDER_LAYER);

    let buttons = [
        require_button::<StanceCycleButton>(&mut app),
        require_button::<AimToggleButton>(&mut app),
        require_button::<FireModeSelectButton>(&mut app),
        require_button::<LevelUpButton>(&mut app),
        require_button::<LevelDownButton>(&mut app),
    ];
    for found in buttons {
        let Some(button) = found else { return };
        assert!(
            app.world().get::<Button>(button).is_some(),
            "an action button must carry Button",
        );
        assert!(
            app.world().get::<Node>(button).is_some(),
            "an action button must carry Node (a bevy_ui node)",
        );
        let on_world_layer = app
            .world()
            .get::<RenderLayers>(button)
            .is_some_and(|layers| *layers == world_layer);
        assert!(
            !on_world_layer,
            "an action button must NOT be on the WORLD_RENDER_LAYER — it routes to the UI camera",
        );
    }
}

// =================================================================================
// GTW-240 — the ENABLED Flee-battle button ends the persisting battle.
// =================================================================================

/// The caption of `button`'s `Text` child, if present (the `spawn_button` widget puts the
/// label on a `Text` child of the button root, not on the root itself).
fn button_label(app: &App, button: Entity) -> Option<String> {
    let children = app.world().get::<Children>(button)?;
    children
        .iter()
        .find_map(|child| app.world().get::<Text>(child).map(|text| text.0.clone()))
}

// ---------------------------------------------------------------------------------
// AC1 — the bar spawns exactly one ENABLED FleeButton (no DisabledButton) in BattleRunning,
// interactive, labelled "Flee battle".
// ---------------------------------------------------------------------------------

/// AC1 — in the live battle exactly one `FleeButton` is spawned; it carries `Button` +
/// `Interaction` (interactive), is ENABLED (NO `DisabledButton`, unlike the deferred
/// reload / end-turn buttons), and is labelled `"Flee battle"`.
#[test]
fn flee_button_spawns_enabled_in_battle() {
    let mut app = battle_running_app();

    let Some(flee) = require_button::<FleeButton>(&mut app) else {
        return;
    };
    assert!(
        app.world().get::<Button>(flee).is_some(),
        "the flee button must carry Button",
    );
    assert!(
        app.world().get::<Interaction>(flee).is_some(),
        "the flee button must carry Interaction (interactive)",
    );
    assert!(
        app.world().get::<DisabledButton>(flee).is_none(),
        "the flee button must be ENABLED — it must NOT carry DisabledButton",
    );
    assert_eq!(
        button_label(&app, flee).as_deref(),
        Some("Flee battle"),
        "the flee button must be labelled \"Flee battle\"",
    );
}

// ---------------------------------------------------------------------------------
// AC2 — a FleeButton press inserts BattleRunningComplete and the battle ends (the state
// advances to AnimateOut). This is the load-bearing AC.
// ---------------------------------------------------------------------------------

/// AC2 — pressing the flee button inserts the `BattleRunningComplete` end-signal marker and
/// the marker-gated `move_on` advances the machine out of `BattleRunning` to `AnimateOut`
/// (the explicit end requirement 5(b)).
#[test]
fn flee_button_press_ends_battle() {
    let mut app = battle_running_app();
    assert_eq!(
        battlescape_state(&app),
        Some(BattleScapeState::BattleRunning),
        "the harness must start in BattleRunning",
    );

    let Some(flee) = require_button::<FleeButton>(&mut app) else {
        return;
    };
    press_button(&mut app, flee);

    // Let `flee_button_pressed` insert the marker (Update) and `move_on` run (FixedUpdate),
    // then walk until the state leaves BattleRunning.
    let left_battle_running = advance_until(
        &mut app,
        |app| battlescape_state(app) != Some(BattleScapeState::BattleRunning),
        BUDGET,
    );
    assert!(
        left_battle_running,
        "a flee press must advance the machine out of BattleRunning within {BUDGET} updates; \
         last observed BattleScapeState was {:?}",
        battlescape_state(&app),
    );
    assert_eq!(
        battlescape_state(&app),
        Some(BattleScapeState::AnimateOut),
        "a flee press must end the battle by advancing BattleRunning -> AnimateOut",
    );
}

// ---------------------------------------------------------------------------------
// AC3 — a flee press is inert without BattleInProgress (the run_if gate holds).
// ---------------------------------------------------------------------------------

/// AC3 — with the `BattleInProgress` live-battle witness removed, a synthesized flee press
/// neither inserts `BattleRunningComplete` nor advances the state out of `BattleRunning` —
/// the `run_if(resource_exists::<BattleInProgress>)` gate holds (`bevy-traps.md` #1). The
/// REAL flee button entity is pressed (proving the system gate, not merely spawn timing).
#[test]
fn flee_button_inert_without_battle_in_progress() {
    let mut app = battle_running_app();
    let Some(flee) = require_button::<FleeButton>(&mut app) else {
        return;
    };

    // Remove the live-battle witness so the flee system's gate excludes it.
    app.world_mut().remove_resource::<BattleInProgress>();

    press_button(&mut app, flee);
    // Several updates to prove no late insertion.
    for _ in 0..3 {
        app.update();
    }

    assert!(
        app.world()
            .get_resource::<BattleRunningComplete>()
            .is_none(),
        "a flee press with no BattleInProgress must NOT insert BattleRunningComplete",
    );
    assert_eq!(
        battlescape_state(&app),
        Some(BattleScapeState::BattleRunning),
        "a flee press with no BattleInProgress must NOT advance the state out of BattleRunning",
    );
}

// ---------------------------------------------------------------------------------
// AC4 — FleeButton despawns OnExit(BattleRunning) with the bar.
// ---------------------------------------------------------------------------------

/// AC4 — after fleeing and leaving `BattleRunning`, the `FleeButton` is gone: it tore down
/// with the `ActionBarRoot` recursive despawn (`despawn_action_bar`), so it is battle-scoped
/// like every other action-bar button.
#[test]
fn flee_button_despawns_on_exit_battle_running() {
    let mut app = battle_running_app();
    let Some(flee) = require_button::<FleeButton>(&mut app) else {
        return;
    };
    press_button(&mut app, flee);

    let left_battle_running = advance_until(
        &mut app,
        |app| battlescape_state(app) != Some(BattleScapeState::BattleRunning),
        BUDGET,
    );
    assert!(
        left_battle_running,
        "a flee press must advance the machine out of BattleRunning within {BUDGET} updates",
    );
    assert!(
        single_with::<FleeButton>(&mut app).is_none(),
        "the flee button must be despawned once the battle leaves BattleRunning (with the bar)",
    );
}

// ---------------------------------------------------------------------------------
// AC5 — regression: the deferred buttons stay disabled, flee stays enabled, and the
// sim-act buttons are unchanged. The `deferred_buttons_are_disabled_and_emit_nothing`
// test above covers the deferred/sim-act contract; this pins the enabled/disabled split
// now that FleeButton has been added (it did not perturb the bar contract).
// ---------------------------------------------------------------------------------

/// AC5 — adding the flee button did NOT re-enable the deferred buttons: `ReloadButton` and
/// `EndTurnButton` still carry `DisabledButton`, while the new `FleeButton` is ENABLED
/// (carries NO `DisabledButton`). The full deferred-emit-nothing + sim-act regression is
/// `deferred_buttons_are_disabled_and_emit_nothing`.
#[test]
fn deferred_buttons_stay_disabled_after_flee_added() {
    let mut app = battle_running_app();

    let Some(reload) = require_button::<ReloadButton>(&mut app) else {
        return;
    };
    let Some(end_turn) = require_button::<EndTurnButton>(&mut app) else {
        return;
    };
    let Some(flee) = require_button::<FleeButton>(&mut app) else {
        return;
    };

    assert!(
        app.world().get::<DisabledButton>(reload).is_some(),
        "the reload button must STAY DisabledButton after FleeButton was added",
    );
    assert!(
        app.world().get::<DisabledButton>(end_turn).is_some(),
        "the end-turn button must STAY DisabledButton after FleeButton was added",
    );
    assert!(
        app.world().get::<DisabledButton>(flee).is_none(),
        "the flee button is ENABLED — it must NOT carry DisabledButton",
    );
}
