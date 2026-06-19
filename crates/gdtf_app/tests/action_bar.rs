//! GTW-228 / GTW-265 / GTW-267 (GTW-48 S9 / 222c): headless integration tests for the
//! themed UI action-bar — the `gdtf_app`-side button surface over the SAME 222a
//! act-intent seam the keyboard surface writes, plus the GTW-267 Stance and GTW-265 Mode
//! 3-toggle sub-panels that REPLACED the blind stance cycle and the fire-mode popup
//! picker.
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
//! - AC3 — an aim button press writes the SAME `*Requested` the equivalent 222a intent
//!   does, byte-for-byte (the `acts.rs` AC5 parity idiom); the level buttons mutate
//!   `ActiveLevel` like the level intent.
//! - GTW-267 — the Stance sub-panel: selecting a ganger marks its current stance toggle
//!   `ActiveButton`; pressing Prone direct-sets stance Prone (a `SetStanceRequested`) and
//!   the active mark moves; the three are mutually exclusive.
//! - GTW-265 / GTW-284 — the Mode sub-panel: the THREE FIXED mode toggles are spawned once
//!   and MUTATED in place (GTW-284: never despawned/respawned). A Single+Burst weapon shows
//!   the Single + Burst toggles `Visible` and Full `Hidden`; selecting a ganger marks its
//!   active mode toggle `ActiveButton`; clicking Burst sets `SelectedFireMode` to that
//!   weapon's burst spec and the active mark moves. A weapon change keeps the toggle entity
//!   ids STABLE and only flips their `Visibility`.
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
//! (in-engine evidence); these synthesized-`Interaction` parity tests are the strong
//! evidence.

use bevy::{
    camera::visibility::RenderLayers,
    ecs::entity::Entity,
    prelude::*,
    state::state::State,
    ui::{Display, Interaction, Node, widget::Button},
};
use gdtf_app::test_support::{
    AimToggleButton, AppState, BattleRunningComplete, BattleScapeState, EndTurnButton, FleeButton,
    LevelDownButton, LevelUpButton, LoadedSituation, ModeBurstButton, ModeFullButton,
    ModePanelRoot, ModeSingleButton, RunningState, StanceKneelingButton, StanceProneButton,
    StanceStandingButton,
};
use gdtf_battle_input::{ActIntent, PendingActIntent, SelectedFireMode, SelectedShooter};
use gdtf_battle_presenter::{ActiveLevel, WORLD_RENDER_LAYER};
use gdtf_battle_sim::{
    Aiming, ArmorFloor, ArmorHardness, ArmorIntegrity, ArmorPiece, ArmorProtection, ArmorType,
    BattleInProgress, Cell, CellLevel, Direction, Facing, Faction, FireMode, FireModeSpec,
    GangerName, GangerSpawn, Hp, HpMax, Level, LifeState, Luck, Magazine, ModeConeMult, ModeKind,
    ModeShots, ModeTuPercent, ReloadTu, Shooting, Situation, SourceArmor, Stance, StanceKind,
    Toughness, Tu, TuMax, Wounds, WoundsMax,
    acts::{EndTurnRequested, SetAimingRequested, SetStanceRequested},
    tuning::CombatTuning,
    weapon::{
        Accuracy, BaseSpread, DamageType, FatalBias, Kickback, MagazineSize, Stable, WeaponDamage,
        WeaponName, WeaponPunch, WeaponRegistry, WeaponShred, WeaponSpec,
    },
};
use gdtf_test_utils::{GdtfTestAppBuilder, advance_until};
use gdtf_ui::{ActiveSegment, DisabledButton, SegmentIndex, theme::default_theme};

/// A budget large enough to drive the deep walk into the battlescape (each leaf scene
/// spends a couple of `FixedUpdate` ticks plus transition propagation), bounded so a
/// machine that never reaches the predicate fails instead of hanging (the
/// `battle_running_driver.rs` budget).
const BUDGET: u32 = 96;

/// The number of STABLE control buttons the bar spawns at `OnEnter(BattleRunning)`: the
/// three stance toggles (Stand / Kneel / Prone), the aim toggle, and the two level
/// buttons. The Mode sub-panel's per-mode toggles are built on selection (none at spawn),
/// and the two DEFERRED buttons (reload, end-turn) are counted separately where relevant.
const STABLE_CONTROL_BUTTONS: usize = 6;

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
    // GTW-257: the Load->Intro gate also requires a WeaponRegistry (empty-default
    // situation here, so an empty registry clears the gate).
    app.world_mut().insert_resource(WeaponRegistry::default());
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

/// Counts the entities carrying marker `M`.
fn count_with<M: Component>(app: &mut App) -> usize {
    let mut q = app.world_mut().query_filtered::<Entity, With<M>>();
    q.iter(app.world()).count()
}

/// Whether the SEGMENT carrying marker `M` is the ACTIVE one of its
/// [`SegmentedControl`](gdtf_ui::SegmentedControl) (GTW-277): its
/// [`SegmentIndex`](gdtf_ui::SegmentIndex) equals its parent control's
/// [`ActiveSegment`](gdtf_ui::ActiveSegment).
///
/// The widget-seam equivalent of the old `ActiveButton`-on-toggle check: with the migration
/// to a `SegmentedControl`, the mutually-exclusive active mark is the control's
/// `ActiveSegment` index (driven + repainted by `gdtf_ui`), so "marker M is active" means
/// "M's segment index is the control's active index".
fn segment_is_active<M: Component>(app: &mut App) -> bool {
    let Some(segment) = single_with::<M>(app) else {
        return false;
    };
    let Some(index) = app.world().get::<SegmentIndex>(segment).map(|i| **i) else {
        return false;
    };
    let Some(parent) = app.world().get::<ChildOf>(segment).map(ChildOf::parent) else {
        return false;
    };
    app.world()
        .get::<ActiveSegment>(parent)
        .is_some_and(|active| **active == index)
}

/// The [`Display`] of the SEGMENT carrying marker `M`, if exactly one exists (GTW-277: the
/// mode segments are MUTATED in place — their `Display` (Flex / None), not their presence,
/// encodes the offered modes; the GTW-284 mutate-not-churn invariant adapted to the widget).
fn segment_display<M: Component>(app: &mut App) -> Option<Display> {
    single_with::<M>(app).and_then(|e| app.world().get::<Node>(e).map(|n| n.display))
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

/// One fire-mode spec of an explicit [`ModeKind`] — the kind is what a toggle / the cycle
/// identifies a mode by (magnitudes arbitrary, not pinned tuning — the `acts.rs`
/// precedent).
const fn spec(kind: ModeKind, tu_percent: f32, shots: u16) -> FireModeSpec {
    FireModeSpec::new(
        kind,
        ModeConeMult::new(1.0),
        ModeTuPercent::new(tu_percent),
        ModeShots::new(shots),
    )
}

/// A three-mode `[Single, Burst, Full]` selector with three distinct modes.
fn sbf_selector() -> FireMode {
    FireMode::new(vec![
        spec(ModeKind::Single, 0.2, 1),
        spec(ModeKind::Burst, 0.4, 3),
        spec(ModeKind::Full, 0.7, 6),
    ])
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

/// Collected `EndTurnRequested` messages (GTW-309 probe).
#[derive(Resource, Default)]
struct EndTurnProbe(Vec<EndTurnRequested>);

/// Adds the `EndTurnRequested` probe, running AFTER the intent drain so it observes the same
/// update's emitted message (the `add_probes` idiom). Its own `MessageReader` cursor is
/// independent of the sim's `dispatch_end_turn`, so it reads every message the drain wrote.
fn add_end_turn_probe(app: &mut App) {
    app.world_mut().insert_resource(EndTurnProbe::default());
    app.add_systems(
        Update,
        (|mut r: MessageReader<EndTurnRequested>, mut p: ResMut<EndTurnProbe>| {
            p.0.extend(r.read().copied());
        })
        .after(gdtf_battle_input::dispatch_act_intents),
    );
}

/// The collected `EndTurnRequested` messages.
fn end_turns(app: &App) -> Vec<EndTurnRequested> {
    app.world()
        .get_resource::<EndTurnProbe>()
        .map(|p| p.0.clone())
        .unwrap_or_default()
}

/// Spawns a ganger carrying exactly the components the act / panel systems read
/// (`Stance`/`Facing`/`Aiming`/`FireMode`) and SELECTS it via the `SelectedShooter`
/// resource — the selection the act drain + the sub-panels read. Returns its entity. (The
/// act surface only reads `*SelectedShooter`, so setting the resource directly is the
/// faithful, minimal selection for these button tests; the cursor-click selection path is
/// covered in `gdtf_battle_input`'s `acts.rs`.)
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
/// STABLE control (three stance toggles, aim, and the two level buttons), each carrying
/// `Button` + `Interaction`, and once the battle leaves `BattleRunning` they are
/// despawned.
#[test]
fn action_bar_spawns_in_battle_and_despawns_outside() {
    let mut app = battle_running_app();

    // Each stable-control marker resolves to exactly one entity carrying Button +
    // Interaction (interactive).
    let buttons = [
        require_button::<StanceStandingButton>(&mut app),
        require_button::<StanceKneelingButton>(&mut app),
        require_button::<StanceProneButton>(&mut app),
        require_button::<AimToggleButton>(&mut app),
        require_button::<LevelUpButton>(&mut app),
        require_button::<LevelDownButton>(&mut app),
    ];
    let present = buttons.iter().filter(|b| b.is_some()).count();
    assert_eq!(
        present, STABLE_CONTROL_BUTTONS,
        "the bar spawns exactly one button per stable control",
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
        single_with::<StanceStandingButton>(&mut app).is_none(),
        "the action bar must be despawned once the battle leaves BattleRunning",
    );
}

// ---------------------------------------------------------------------------------
// AC3 — a button press writes the SAME *Requested the equivalent intent does,
// byte-for-byte (over the REAL 222a drain).
// ---------------------------------------------------------------------------------

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

// =================================================================================
// GTW-267 — the Stance 3-toggle sub-panel (replaces the blind cycle).
// =================================================================================

/// GTW-267 — selecting an armed ganger marks ITS current stance toggle `ActiveButton`
/// and ONLY that one (mutually exclusive); a fresh selection in another stance moves the
/// mark. Discriminating: a panel with no sync (the old blind cycle) would mark none.
#[test]
fn stance_panel_marks_current_stance_active() {
    let mut app = battle_running_app();
    // Select a KNEELING ganger.
    arm_and_select(
        &mut app,
        sbf_selector(),
        StanceKind::Crouching,
        Direction::North,
    );
    app.update();

    assert!(
        segment_is_active::<StanceKneelingButton>(&mut app),
        "the selected ganger's current stance (kneel) segment must be the active segment",
    );
    assert!(
        !segment_is_active::<StanceStandingButton>(&mut app)
            && !segment_is_active::<StanceProneButton>(&mut app),
        "the other two stance segments must NOT be active (mutually exclusive)",
    );

    // Selecting a PRONE ganger moves the active mark to Prone.
    arm_and_select(
        &mut app,
        sbf_selector(),
        StanceKind::Prone,
        Direction::North,
    );
    app.update();
    assert!(
        segment_is_active::<StanceProneButton>(&mut app),
        "selecting a prone ganger must move the active mark to the Prone segment",
    );
    assert!(
        !segment_is_active::<StanceStandingButton>(&mut app)
            && !segment_is_active::<StanceKneelingButton>(&mut app),
        "the active stance mark must be exclusive after the re-selection",
    );
}

/// GTW-267 — pressing the Prone toggle DIRECT-sets the actor's stance to Prone (one
/// `SetStanceRequested` for `*SelectedShooter` with `StanceKind::Prone`, byte-for-byte
/// EQUAL to the direct `SetStance(Prone)` intent), regardless of the current stance — NOT
/// a blind cycle step.
#[test]
fn prone_toggle_sets_stance_prone_directly() {
    let mut app = battle_running_app();
    add_probes(&mut app);
    let ganger = arm_and_select(
        &mut app,
        sbf_selector(),
        StanceKind::Standing,
        Direction::North,
    );
    let Some(prone) = require_button::<StanceProneButton>(&mut app) else {
        return;
    };
    press_button(&mut app, prone);
    app.update();

    let via_button = stances(&app);
    assert_eq!(
        via_button.len(),
        1,
        "one SetStanceRequested via the Prone toggle",
    );
    assert_eq!(via_button[0].actor, ganger, "actor = *SelectedShooter");
    assert_eq!(
        via_button[0].stance,
        StanceKind::Prone,
        "the Prone toggle DIRECT-sets the prone posture (not a cycle step)",
    );

    // Byte-for-byte equal to the direct SetStance(Prone) intent over the SAME seam.
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
        .push(ActIntent::SetStance(StanceKind::Prone));
    app2.update();
    let via_intent = stances(&app2);
    assert_eq!(via_intent.len(), 1, "one SetStanceRequested via the intent");
    assert_eq!(
        via_button[0], via_intent[0],
        "the Prone toggle and the direct SetStance intent must produce byte-for-byte equal \
         SetStanceRequested",
    );
}

/// GTW-277 (new — the `SegmentSelected` → `SetStance` index→kind MAPPING) — selecting EACH
/// stance segment pushes the matching `ActIntent::SetStance(kind)` (Stand → Standing, Kneel
/// → Crouching, Prone → Prone), proving the `stance_segment_intent` listener maps the
/// widget's segment INDEX to the right `StanceKind`, not just the Prone case.
#[test]
fn each_stance_segment_sets_its_stance() {
    for (label, marker_press, start, expected) in [
        // Start from a stance DIFFERENT from the target so pressing the segment is a REAL
        // active-segment change (re-pressing the already-active segment is a widget no-op,
        // the GTW-276 set_if_neq behavior).
        (
            "Stand",
            StanceSegment::Standing,
            StanceKind::Prone,
            StanceKind::Standing,
        ),
        (
            "Kneel",
            StanceSegment::Kneeling,
            StanceKind::Standing,
            StanceKind::Crouching,
        ),
        (
            "Prone",
            StanceSegment::Prone,
            StanceKind::Standing,
            StanceKind::Prone,
        ),
    ] {
        let mut app = battle_running_app();
        add_probes(&mut app);
        let ganger = arm_and_select(&mut app, sbf_selector(), start, Direction::North);
        // Settle the segment tagging + the active-segment sync to the ganger's STARTING
        // stance, so pressing the target segment is a real change (emits SegmentSelected →
        // SetStance).
        app.update();
        app.update();
        let Some(segment) = marker_press.entity(&mut app) else {
            return;
        };
        press_button(&mut app, segment);
        app.update();

        let pushed = stances(&app);
        assert_eq!(
            pushed.len(),
            1,
            "{label} segment must push exactly one SetStanceRequested",
        );
        assert_eq!(pushed[0].actor, ganger, "{label}: actor = *SelectedShooter");
        assert_eq!(
            pushed[0].stance, expected,
            "the {label} segment must direct-set {expected:?}",
        );
    }
}

/// Picks the right stance-segment entity by its per-stance marker for
/// [`each_stance_segment_sets_its_stance`] (a small dispatch so the loop can press each).
enum StanceSegment {
    Standing,
    Kneeling,
    Prone,
}

impl StanceSegment {
    /// The single entity carrying this stance's segment marker, if exactly one exists.
    fn entity(&self, app: &mut App) -> Option<Entity> {
        match self {
            Self::Standing => single_with::<StanceStandingButton>(app),
            Self::Kneeling => single_with::<StanceKneelingButton>(app),
            Self::Prone => single_with::<StanceProneButton>(app),
        }
    }
}

// =================================================================================
// GTW-265 — the Mode 3-toggle sub-panel (replaces the popup picker).
// =================================================================================

/// GTW-265 / GTW-277 / GTW-284 — the three FIXED mode SEGMENTS always exist; a Single+Burst
/// weapon SHOWS the Single + Burst segments (`Display::Flex`) and HIDES Full
/// (`Display::None`, the mode it does not offer). The active mark sits on the live mode
/// (`single()` default on selection) — the control's `ActiveSegment`. (Adapted to the
/// GTW-277 widget seam: "visible" = `Display::Flex`, "active" = the control's active
/// segment; the only-offered-visible + active-mark CONTRACTS are unchanged.)
#[test]
fn mode_panel_spawns_only_offered_modes_and_marks_active() {
    let single = spec(ModeKind::Single, 0.2, 1);
    let burst = spec(ModeKind::Burst, 0.4, 3);
    let mut app = battle_running_app();
    arm_and_select(
        &mut app,
        FireMode::new(vec![single, burst]),
        StanceKind::Standing,
        Direction::North,
    );
    // Settle the selection default + the rebuild (.after ApplyTheme) + the active sync.
    app.update();
    app.update();

    // GTW-277/284: the three fixed segments always exist (one each); only Display changes.
    assert_eq!(
        count_with::<ModeSingleButton>(&mut app),
        1,
        "the fixed Single mode segment exists exactly once",
    );
    assert_eq!(
        count_with::<ModeBurstButton>(&mut app),
        1,
        "the fixed Burst mode segment exists exactly once",
    );
    assert_eq!(
        count_with::<ModeFullButton>(&mut app),
        1,
        "the fixed Full mode segment exists exactly once (it is collapsed, not despawned)",
    );
    assert_eq!(
        segment_display::<ModeSingleButton>(&mut app),
        Some(Display::Flex),
        "a Single+Burst weapon shows the Single mode segment (Display::Flex)",
    );
    assert_eq!(
        segment_display::<ModeBurstButton>(&mut app),
        Some(Display::Flex),
        "a Single+Burst weapon shows the Burst mode segment (Display::Flex)",
    );
    assert_eq!(
        segment_display::<ModeFullButton>(&mut app),
        Some(Display::None),
        "a Single+Burst weapon HIDES the Full mode segment (Display::None — not offered)",
    );

    // The default-on-select mode is single(), so the Single segment is the active one.
    assert!(
        segment_is_active::<ModeSingleButton>(&mut app),
        "the active mode segment must be the live SelectedFireMode (single() default)",
    );
    assert!(
        !segment_is_active::<ModeBurstButton>(&mut app),
        "the non-active mode segment must NOT be marked active",
    );
}

/// GTW-265 / GTW-277 — selecting the Burst SEGMENT sets `SelectedFireMode` to that weapon's
/// burst spec (read-back identity, never fabricated) and the active mark moves Single ->
/// Burst. Driving the widget = pressing the segment (its `Interaction` → `Pressed`), which
/// `gdtf_ui`'s `select_segment_on_press` turns into a `SegmentSelected` message the
/// `mode_segment_write` listener reads — the SAME downstream contract as the old toggle press.
#[test]
fn clicking_burst_toggle_sets_mode_and_moves_active_mark() {
    let single = spec(ModeKind::Single, 0.2, 1);
    let burst = spec(ModeKind::Burst, 0.4, 3);
    let full = spec(ModeKind::Full, 0.7, 6);
    let mut app = battle_running_app();
    arm_and_select(
        &mut app,
        FireMode::new(vec![single, burst, full]),
        StanceKind::Standing,
        Direction::North,
    );
    // Settle default-on-select + the segment rebuild + the active sync.
    app.update();
    app.update();

    // Pre-condition: Single is the active mode by default.
    assert_eq!(
        selected_mode(&app),
        Some(single),
        "the default-on-select mode must be single()",
    );

    let Some(burst_segment) = require_button::<ModeBurstButton>(&mut app) else {
        return;
    };
    press_button(&mut app, burst_segment);
    app.update();

    assert_eq!(
        selected_mode(&app),
        Some(burst),
        "selecting the Burst segment sets SelectedFireMode to the weapon's burst spec \
         (read-back, not fabricated)",
    );
    assert!(
        segment_is_active::<ModeBurstButton>(&mut app),
        "the active mark must move to the Burst segment",
    );
    assert!(
        !segment_is_active::<ModeSingleButton>(&mut app),
        "the Single segment must no longer be active after Burst is chosen",
    );
}

/// GTW-284 AC1 / GTW-277 — on a weapon/selection change the mode SEGMENTS are MUTATED in
/// place, never despawned/respawned: the segment `Entity` ids stay STABLE across the change
/// and only their `Display` flips to match the new weapon's offered modes (3-mode weapon →
/// all `Display::Flex`; re-select a Single-only weapon → Single Flex, Burst + Full
/// `Display::None`). This is the GTW-284 stable-id invariant preserved through the migration
/// to a `gdtf_ui` `SegmentedControl` — the control is spawned ONCE with all 3 segments and
/// per-segment visibility is toggled via `set_segment_visible`, never a respawn.
///
/// Pin-discriminating: a despawn/respawn body would change the segment ids on the
/// re-selection (failing the id-stability asserts) and would leave the Burst/Full segments
/// absent rather than `Display::None`.
#[test]
fn mode_toggles_mutate_in_place_keeping_stable_ids() {
    let single = spec(ModeKind::Single, 0.2, 1);
    let burst = spec(ModeKind::Burst, 0.4, 3);
    let full = spec(ModeKind::Full, 0.7, 6);

    let mut app = battle_running_app();
    arm_and_select(
        &mut app,
        FireMode::new(vec![single, burst, full]),
        StanceKind::Standing,
        Direction::North,
    );
    app.update();
    app.update();

    // Capture the three fixed segment ids under weapon A (all three modes offered → Flex).
    let Some(single_a) = single_with::<ModeSingleButton>(&mut app) else {
        return;
    };
    let Some(burst_a) = single_with::<ModeBurstButton>(&mut app) else {
        return;
    };
    let Some(full_a) = single_with::<ModeFullButton>(&mut app) else {
        return;
    };
    assert_eq!(
        segment_display::<ModeSingleButton>(&mut app),
        Some(Display::Flex),
        "weapon A (Single+Burst+Full) shows the Single segment (Display::Flex)",
    );
    assert_eq!(
        segment_display::<ModeBurstButton>(&mut app),
        Some(Display::Flex),
        "weapon A shows the Burst segment (Display::Flex)",
    );
    assert_eq!(
        segment_display::<ModeFullButton>(&mut app),
        Some(Display::Flex),
        "weapon A shows the Full segment (Display::Flex)",
    );

    // Re-select a 1-mode (Single-only) weapon B: the segments MUTATE (Display flips), the
    // entity ids do NOT change (no despawn/respawn).
    arm_and_select(
        &mut app,
        FireMode::new(vec![single]),
        StanceKind::Standing,
        Direction::North,
    );
    app.update();
    app.update();

    assert_eq!(
        single_with::<ModeSingleButton>(&mut app),
        Some(single_a),
        "the Single mode segment Entity id must be STABLE across the weapon change (no respawn)",
    );
    assert_eq!(
        single_with::<ModeBurstButton>(&mut app),
        Some(burst_a),
        "the Burst mode segment Entity id must be STABLE across the weapon change (no respawn)",
    );
    assert_eq!(
        single_with::<ModeFullButton>(&mut app),
        Some(full_a),
        "the Full mode segment Entity id must be STABLE across the weapon change (no respawn)",
    );

    // Display now matches weapon B's single offered mode.
    assert_eq!(
        segment_display::<ModeSingleButton>(&mut app),
        Some(Display::Flex),
        "weapon B (Single-only) keeps the Single segment shown (Display::Flex)",
    );
    assert_eq!(
        segment_display::<ModeBurstButton>(&mut app),
        Some(Display::None),
        "weapon B (Single-only) HIDES the Burst segment (Display::None — not offered), not despawns it",
    );
    assert_eq!(
        segment_display::<ModeFullButton>(&mut app),
        Some(Display::None),
        "weapon B (Single-only) HIDES the Full segment (Display::None — not offered), not despawns it",
    );
}

// ---------------------------------------------------------------------------------
// GTW-309 — the end-turn button is now LIVE: enabled, and a press pushes the fieldless
// GLOBAL ActIntent::EndTurn, which the ONE drain emits as a fieldless EndTurnRequested
// WITHOUT needing a selection. (GTW-275 removed the Reload deferred stub — reload is a LIVE
// weapon-panel button now — so there is no longer ANY deferred action-bar button.)
// ---------------------------------------------------------------------------------

/// GTW-309 — the end-turn button is ENABLED: it carries NO `DisabledButton`, so the
/// `Without<DisabledButton>` action filter now INCLUDES it (it is no longer a deferred
/// placeholder). The full press→intent→message wiring is exercised by
/// [`end_turn_button_emits_one_end_turn_requested_without_selection`].
#[test]
fn end_turn_button_is_enabled_not_disabled() {
    let mut app = battle_running_app();
    let Some(end_turn) = require_button::<EndTurnButton>(&mut app) else {
        return;
    };
    assert!(
        app.world().get::<DisabledButton>(end_turn).is_none(),
        "the end-turn button must be ENABLED — it must NOT carry DisabledButton (GTW-309)",
    );
    assert!(
        app.world().get::<Button>(end_turn).is_some(),
        "the end-turn button must carry Button (interactive)",
    );
    assert!(
        app.world().get::<Interaction>(end_turn).is_some(),
        "the end-turn button must carry Interaction (interactive)",
    );
}

/// GTW-309 — pressing the end-turn button enqueues exactly one fieldless
/// `ActIntent::EndTurn`, and the ONE `dispatch_act_intents` drain emits exactly one fieldless
/// `EndTurnRequested` from it — with NO `SelectedShooter` set (the global turn signal needs no
/// selection, unlike a per-actor act). This is the load-bearing AC: it drives the REAL stack
/// (button press → 222a seam → the SAME drain the keyboard surface feeds) and asserts the
/// end-to-end message, byte-for-byte equal to the message the direct `ActIntent::EndTurn`
/// pushes (the `acts.rs` parity idiom).
///
/// Pin-discriminating: removing the new `EndTurnButton` arm in `action_bar_button_intents`
/// (or re-adding the `DisabledButton` marker) leaves the queue empty and emits zero messages,
/// failing the `len == 1` asserts.
#[test]
fn end_turn_button_emits_one_end_turn_requested_without_selection() {
    let mut app = battle_running_app();
    add_end_turn_probe(&mut app);
    // Deliberately NO arm_and_select / SelectedShooter — the end-turn intent is a fieldless
    // GLOBAL signal the drain emits unconditionally.

    let Some(end_turn) = require_button::<EndTurnButton>(&mut app) else {
        return;
    };
    press_button(&mut app, end_turn);
    app.update();

    let via_button = end_turns(&app);
    assert_eq!(
        via_button.len(),
        1,
        "pressing the end-turn button must emit exactly one EndTurnRequested (no selection \
         needed)",
    );

    // Byte-for-byte equal to the message the direct ActIntent::EndTurn (the keyboard surface)
    // pushes over the SAME seam — proving the button is a parallel surface, not a divergent
    // emission path.
    let mut app2 = battle_running_app();
    add_end_turn_probe(&mut app2);
    app2.world_mut()
        .resource_mut::<PendingActIntent>()
        .push(ActIntent::EndTurn);
    app2.update();
    let via_intent = end_turns(&app2);
    assert_eq!(
        via_intent.len(),
        1,
        "one EndTurnRequested via the direct ActIntent::EndTurn"
    );
    assert_eq!(
        via_button[0], via_intent[0],
        "the button and the key/intent surface must produce byte-for-byte equal \
         EndTurnRequested",
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

    // Press the stance toggles + aim button (the *Requested-emitting acts) with no
    // selection.
    let act_buttons = [
        single_with::<StanceStandingButton>(&mut app),
        single_with::<StanceProneButton>(&mut app),
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
        require_button::<StanceStandingButton>(&mut app),
        require_button::<AimToggleButton>(&mut app),
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
// interactive, labelled "Flee" (D-D: shortened from "Flee battle").
// ---------------------------------------------------------------------------------

/// AC1 — in the live battle exactly one `FleeButton` is spawned; it carries `Button` +
/// `Interaction` (interactive), is ENABLED (NO `DisabledButton`, unlike the deferred
/// reload / end-turn buttons), and is labelled `"Flee"` (D-D: shortened from "Flee battle").
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
        Some("Flee"),
        "the flee button must be labelled \"Flee\" (D-D)",
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
// GTW-309 / GTW-240 — the end-turn and flee buttons are both ENABLED, distinct controls.
// ---------------------------------------------------------------------------------

/// GTW-309 / GTW-240 — the end-turn button (GTW-309 made it LIVE) and the flee button are
/// both ENABLED (neither carries `DisabledButton`) and are distinct entities: enabling the
/// end-turn button did not disturb the flee button, and vice versa. The full end-turn
/// press→emit wiring is `end_turn_button_emits_one_end_turn_requested_without_selection`;
/// the flee end-battle wiring is `flee_button_press_ends_battle`. (GTW-275: the Reload
/// deferred stub is GONE — reload is a LIVE weapon-panel button now.)
#[test]
fn end_turn_and_flee_buttons_are_both_enabled() {
    let mut app = battle_running_app();

    let Some(end_turn) = require_button::<EndTurnButton>(&mut app) else {
        return;
    };
    let Some(flee) = require_button::<FleeButton>(&mut app) else {
        return;
    };

    assert_ne!(
        end_turn, flee,
        "the end-turn and flee buttons must be distinct entities",
    );
    assert!(
        app.world().get::<DisabledButton>(end_turn).is_none(),
        "the end-turn button is ENABLED since GTW-309 — it must NOT carry DisabledButton",
    );
    assert!(
        app.world().get::<DisabledButton>(flee).is_none(),
        "the flee button is ENABLED — it must NOT carry DisabledButton",
    );
}

// =================================================================================
// GTW-272 + GTW-273 — Mode-panel visibility: hide the Mode panel when nothing armed is
// selected (AC3). Pin-discriminating headless tests. (GTW-298 RELOCATED the Mode + Stance
// sub-panels OUT of the action bar into the weapon cluster, so the old Mode-LEFT-of-Stance
// bar-child-order test is gone — that layout is now covered by the weapon-cluster structure
// test in `weapon_panel.rs`.)
//
// AC1 (fit-content / no clipping) is largely a VISUAL check → in-engine QA per
// verification.md #3; the headless slice asserts the explicit fit-content `Node`
// fields the fix sets (no brittle pixel pin), see `action_bar_root_fits_contents`.
// =================================================================================

/// The weapon key the real-flow armed ganger references — present in [`armed_registry`].
const PLAYER_WEAPON_KEY: &str = "test-weapon";

/// The gang the player controls in these tests — `Situation::player_faction` defaults to
/// gang `0`, so the armed ganger is faction `0` for `auto_select_first_player_ganger` to
/// pick it via the real flow (the `battle_running_driver.rs` precedent).
const PLAYER_FACTION: u8 = 0;

/// A [`WeaponRegistry`] holding the one [`PLAYER_WEAPON_KEY`] weapon the armed real-flow
/// ganger references (stands in for the `Load`-built registry, GTW-257, so the Generation
/// setup arms the ganger). Its `fire_mode` offers a single Single mode, so an armed
/// selection yields one Mode toggle → the Mode panel is `Visible` (the
/// `battle_running_driver.rs::weapon_registry` shape).
fn armed_registry() -> WeaponRegistry {
    WeaponRegistry::new([(
        WeaponName::new(PLAYER_WEAPON_KEY.to_owned()),
        WeaponSpec {
            base_spread: BaseSpread::new(0.25),
            accuracy:    Accuracy::new(1.0),
            kickback:    Kickback::new(0.4),
            fatal_bias:  FatalBias::new(0.0),
            damage:      WeaponDamage::new(12),
            punch:       WeaponPunch::new(5),
            shred:       WeaponShred::new(3),
            damage_type: DamageType::Kinetic,
            magazine:    Magazine::loaded(MagazineSize::new(30), ReloadTu::new(12)),
            fire_mode:   FireMode::new(vec![spec(ModeKind::Single, 0.5, 1)]),
            stable:      Stable::new(false),
        },
    )])
}

/// An arbitrary roster armor record (distinct per-part magnitudes, NOT shipped tuning —
/// the `battle_running_driver.rs::arbitrary_armor` shape).
const fn arbitrary_armor() -> SourceArmor {
    SourceArmor::uniform(ArmorPiece::new(
        ArmorFloor::new(0),
        ArmorProtection::new(1),
        ArmorIntegrity::new(2),
        ArmorHardness::new(3),
        ArmorType::DEFAULT,
    ))
}

/// A one-ganger real situation: a single armed faction-[`PLAYER_FACTION`] ganger the
/// `SetupBattleRequested` (sent on `OnEnter(Generation)`) spawns via `setup_battle` and
/// `auto_select_first_player_ganger` then selects through the REAL flow — NO hand-inserted
/// `SelectedShooter`. Its weapon key is present in [`armed_registry`], so setup arms it
/// with a `FireMode`, which `rebuild_mode_buttons` reads to show the Mode panel.
fn armed_player_situation() -> Situation {
    Situation {
        gangers: vec![GangerSpawn {
            at:         CellLevel::new(Cell::new(2, 5), Level::new(0)),
            name:       GangerName::new("Alex Mercer".to_owned()),
            faction:    Faction::new(PLAYER_FACTION),
            facing:     Facing::new(Direction::East),
            stance:     Stance::new(StanceKind::Standing),
            aiming:     Aiming::new(false),
            hp:         Hp::new(40),
            hp_max:     HpMax::new(40),
            wounds:     Wounds::new(3),
            wounds_max: WoundsMax::new(3),
            tu:         Tu::new(60),
            tu_max:     TuMax::new(60),
            life_state: LifeState::Alive,
            shooting:   Shooting::new(3.0),
            toughness:  Toughness::new(3.0),
            luck:       Luck::new(1.0),
            armor:      arbitrary_armor(),
            weapon:     WeaponName::new(PLAYER_WEAPON_KEY.to_owned()),
        }],
        ..Situation::new()
    }
}

/// Builds the headless walk app exactly like [`walk_app`], but with a real
/// [`LoadedSituation`] for the Generation setup to pour into the world (so a real player
/// ganger is spawned + auto-selected) and the matching [`armed_registry`] so setup can arm
/// it. Mirrors the `battle_running_driver.rs` real-flow harness.
fn walk_app_with_situation(situation: Situation) -> App {
    let mut app = GdtfTestAppBuilder::new()
        .starting_in(AppState::Running)
        .build();
    app.world_mut().insert_resource(default_theme());
    app.world_mut().insert_resource(CombatTuning::default());
    app.world_mut().insert_resource(armed_registry());
    app.world_mut().insert_resource(LoadedSituation(situation));
    app
}

/// Drives the real-situation walk to `BattleRunning` and returns the app, asserting the
/// descent succeeded — the live battle where the bar is spawned AND the real auto-select
/// has run on the spawned player ganger.
fn battle_running_app_with_situation(situation: Situation) -> App {
    let mut app = walk_app_with_situation(situation);
    assert!(
        drive_to_battle_running(&mut app),
        "the real-situation walk should reach BattleScapeState::BattleRunning within {BUDGET} \
         updates; last observed BattleScapeState was {:?}",
        battlescape_state(&app),
    );
    app
}

/// The `ModePanelRoot`'s current [`Visibility`], if exactly one exists.
fn mode_panel_visibility(app: &mut App) -> Option<Visibility> {
    single_with::<ModePanelRoot>(app)
        .and_then(|panel| app.world().get::<Visibility>(panel).copied())
}

// ---------------------------------------------------------------------------------
// AC3 — the Mode panel is Hidden when nothing armed is selected, Visible when an armed
// player ganger is selected (via the REAL selection flow — no hand-inserted selection).
// ---------------------------------------------------------------------------------

/// GTW-273 AC3 (hidden) — with NO armed selection reached via the REAL flow (the empty
/// situation spawns no gangers, so `auto_select_first_player_ganger` selects nothing), the
/// `ModePanelRoot` is `Visibility::Hidden` — no empty Mode box. Pin-discriminating:
/// reverting the visibility toggle (leaving the panel `Inherited`/visible) fails this.
#[test]
fn mode_panel_hidden_when_nothing_armed_selected() {
    let mut app = battle_running_app();
    // Settle the initial selection (stays None — no gangers) + the rebuild's no-op /
    // unarmed branch, which sets the panel Hidden.
    app.update();
    app.update();

    assert_eq!(
        mode_panel_visibility(&mut app),
        Some(Visibility::Hidden),
        "with nothing armed selected, the Mode panel must be Hidden (no empty Mode box)",
    );
}

/// GTW-273 AC3 (visible) — when an armed player ganger is selected via the REAL flow (the
/// real `LoadedSituation` setup spawns + arms it, `auto_select_first_player_ganger` picks
/// it — NO hand-inserted `SelectedShooter`), the `ModePanelRoot` is `Visibility::Visible`
/// (its weapon offers a mode → a toggle to show). Pin-discriminating: reverting the
/// visibility toggle (leaving it Hidden) fails this.
#[test]
fn mode_panel_visible_when_armed_player_ganger_selected() {
    let mut app = battle_running_app_with_situation(armed_player_situation());
    // Settle the real auto-select (fills SelectedShooter with the player ganger) + the
    // rebuild (.after ApplyTheme) which, for an armed selection with modes, sets Visible.
    app.update();
    app.update();

    // The selection was filled by the REAL auto-select path (not hand-inserted).
    assert!(
        app.world()
            .get_resource::<SelectedShooter>()
            .is_some_and(|s| s.is_some()),
        "precondition: the real auto-select must have filled SelectedShooter with the player \
         ganger (no hand-inserted selection)",
    );
    // The armed weapon offers exactly one mode → exactly one toggle is built.
    assert_eq!(
        count_with::<ModeSingleButton>(&mut app),
        1,
        "the armed player ganger's single-mode weapon must build one Mode toggle",
    );
    assert_eq!(
        mode_panel_visibility(&mut app),
        Some(Visibility::Visible),
        "with an armed player ganger selected, the Mode panel must be Visible",
    );
}

// ---------------------------------------------------------------------------------
// AC1 — the fit-content fix sets explicit Node fields on the bar root (height Auto; the bar
// grows upward from the bottom). Headless assert of the fields the fix sets, NOT a brittle
// pixel pin (the fit-content render itself is in-engine QA).
// ---------------------------------------------------------------------------------

/// GTW-272 AC1 / GTW-298 / D-C — the top action bar SHRINK-WRAPS its contents and sits at the
/// TOP-MIDDLE of the window. D-C (2026-06-18 screenshot review) split the bar into a TRANSPARENT
/// full-window-width centering WRAPPER (the `ActionBarRoot`) holding a compact, fit-content bordered
/// PANEL of the four buttons:
///
/// - the compact PANEL (the `LevelUp` button's direct parent) has `width: Auto` — it FITS its
///   contents rather than spanning the full window (the old `left:0 + right:0` stretch is gone) —
///   and `height: Auto` (fit-content vertically too);
/// - the WRAPPER (the panel's parent) is the full window WIDTH (`Vw(100)`) anchored to the screen
///   TOP (`top: 0`) and CENTRES the panel horizontally (`justify_content: Center`), anchoring it to
///   the top edge (`align_items: FlexStart`).
///
/// The fit-content / centred RENDER is an in-engine VISUAL check (verification.md #3); this pins
/// the explicit `Node` fields the layout sets (NO brittle pixel pin — only unit-kind asserts). A
/// revert to the full-width stretch (panel `width: Percent`/`right: 0`) fails the `width: Auto`
/// assert; a revert to a non-centred / non-full-width wrapper fails the wrapper asserts.
#[test]
fn action_bar_root_fits_contents_and_is_top_centered() {
    let mut app = battle_running_app();

    // The compact PANEL is the parent of the LevelUp button (a remaining action-bar control).
    let Some(level_up) = require_button::<LevelUpButton>(&mut app) else {
        return;
    };
    let Some(panel) = app.world().get::<ChildOf>(level_up).map(ChildOf::parent) else {
        return;
    };
    let Some(panel_node) = app.world().get::<Node>(panel) else {
        return;
    };
    assert_eq!(
        panel_node.width,
        Val::Auto,
        "the compact button panel must FIT its contents (width Auto, NOT a full-width stretch)",
    );
    assert_eq!(
        panel_node.height,
        Val::Auto,
        "the compact button panel must FIT its contents vertically too (height Auto)",
    );

    // The WRAPPER (the panel's parent, the `ActionBarRoot`) is the full-window-width centering row.
    let Some(wrapper) = app.world().get::<ChildOf>(panel).map(ChildOf::parent) else {
        return;
    };
    let Some(wrapper_node) = app.world().get::<Node>(wrapper) else {
        return;
    };
    assert_eq!(
        wrapper_node.width,
        Val::Vw(100.0),
        "the centering wrapper spans the full window WIDTH (Vw 100) so it can centre the panel",
    );
    assert_eq!(
        wrapper_node.justify_content,
        JustifyContent::Center,
        "the centering wrapper CENTRES the panel horizontally (top-middle) — D-C",
    );
    assert_eq!(
        wrapper_node.align_items,
        AlignItems::FlexStart,
        "the centering wrapper anchors the panel to its TOP edge (FlexStart)",
    );
    assert_eq!(
        wrapper_node.top,
        Val::Px(0.0),
        "the centering wrapper stays anchored to the screen TOP (GTW-298 / D-C)",
    );
}
