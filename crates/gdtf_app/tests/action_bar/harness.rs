//! App-driving harness + shared query vocabulary for the action-bar suite.

use bevy::{
    ecs::entity::Entity,
    prelude::*,
    state::state::State,
    ui::{Display, Node},
};
use gdtf_app::test_support::{AppState, BattleScapeState, RunningState};
use gdtf_battle_sim::{
    ArmorRegistry, FireMode, FireModeSpec, ModeConeMult, ModeKind, ModeShots, ModeTuPercent,
    injuries::InjuryRegistry, tuning::CombatTuning, weapon::WeaponRegistry,
};
use gdtf_test_utils::{GdtfTestAppBuilder, advance_until};
use gdtf_ui::{ActiveSegment, SegmentIndex, theme::default_theme};

/// A budget large enough to drive the deep walk into the battlescape (each leaf scene
/// spends a couple of `FixedUpdate` ticks plus transition propagation), bounded so a
/// machine that never reaches the predicate fails instead of hanging (the
/// `battle_running_driver.rs` budget).
pub(crate) const BUDGET: u32 = 96;

/// The number of STABLE control buttons the bar spawns at `OnEnter(BattleRunning)`: the
/// three stance toggles (Stand / Kneel / Prone), the aim toggle, and the two level
/// buttons. The Mode sub-panel's per-mode toggles are built on selection (none at spawn),
/// and the two DEFERRED buttons (reload, end-turn) are counted separately where relevant.
pub(crate) const STABLE_CONTROL_BUTTONS: usize = 6;

// ---------------------------------------------------------------------------------
// Harness — drive the real stack to BattleRunning, where the bar is live.
// ---------------------------------------------------------------------------------

/// Reads the current [`BattleScapeState`] if it is active.
pub(crate) fn battlescape_state(app: &App) -> Option<BattleScapeState> {
    app.world()
        .get_resource::<State<BattleScapeState>>()
        .map(|state| *state.get())
}

/// Reads the current [`RunningState`] if it is active.
pub(crate) fn running_state(app: &App) -> Option<RunningState> {
    app.world()
        .get_resource::<State<RunningState>>()
        .map(|state| *state.get())
}

/// Builds the headless walk app, injecting the persistent `Load` resources the machine
/// needs to traverse `Load` (no `AssetServer` under `MinimalPlugins`) — `default_theme()`
/// (which `spawn_action_bar` reads) + `CombatTuning`. No `LoadedSituation` → the empty
/// `Situation::default()` battle is set up, which still makes `BattleInProgress` present
/// in `BattleRunning` (the bar's action-system gate).
pub(crate) fn walk_app() -> App {
    let mut app = GdtfTestAppBuilder::new_with_scene_support()
        .starting_in(AppState::Running)
        .build();
    app.world_mut().insert_resource(default_theme());
    app.world_mut().insert_resource(CombatTuning::default());
    // GTW-257: the Load->Intro gate also requires a WeaponRegistry (empty-default
    // situation here, so an empty registry clears the gate).
    app.world_mut().insert_resource(WeaponRegistry::default());
    // GTW-505: the Load->Intro gate also requires a MeleeWeaponRegistry (empty-default
    // seed stands in for the asset-less resolve, mirroring the WeaponRegistry seed above).
    app.world_mut()
        .insert_resource(gdtf_battle_sim::weapon::MeleeWeaponRegistry::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::equipment::attachments::AttachmentRegistry::default());
    // GTW-269: the Load->Intro gate also requires an ArmorRegistry; the empty-default
    // situation has zero gangers, so an empty registry clears the gate and the setup
    // resolves no armor keys.
    app.world_mut().insert_resource(ArmorRegistry::default());
    app.world_mut().insert_resource(InjuryRegistry::default());
    // GTW-415: the Load→Intro gate also requires a GangRegistry; empty clears it.
    app.world_mut()
        .insert_resource(gdtf_battle_sim::ganger::GangRegistry::default());
    // GTW-489: the NEW gate-blocking PrefabRegistry; empty clears it.
    app.world_mut()
        .insert_resource(gdtf_battle_sim::level::PrefabRegistry::default());
    // GTW-487: the NEW gate-blocking TerrainDefRegistry + UuidThemeRegistry.
    app.world_mut()
        .insert_resource(gdtf_battle_sim::terrain::def::TerrainDefRegistry::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::level::UuidThemeRegistry::default());
    app
}

/// Drives the app from `Running`/`Menu` down to the first update on which
/// [`BattleScapeState::BattleRunning`] is active (the player at the menu picks the
/// Battlescape transition). Returns whether it was reached.
pub(crate) fn drive_to_battle_running(app: &mut App) -> bool {
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
pub(crate) fn battle_running_app() -> App {
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
pub(crate) fn single_with<M: Component>(app: &mut App) -> Option<Entity> {
    let mut q = app.world_mut().query_filtered::<Entity, With<M>>();
    let found: Vec<Entity> = q.iter(app.world()).collect();
    match found.as_slice() {
        [one] => Some(*one),
        _ => None,
    }
}

/// Counts the entities carrying marker `M`.
pub(crate) fn count_with<M: Component>(app: &mut App) -> usize {
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
pub(crate) fn segment_is_active<M: Component>(app: &mut App) -> bool {
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
pub(crate) fn segment_display<M: Component>(app: &mut App) -> Option<Display> {
    single_with::<M>(app).and_then(|e| app.world().get::<Node>(e).map(|n| n.display))
}

/// Asserts exactly one button carrying marker `M` exists and returns it, so callers can
/// `let Some(b) = require_button::<M>(..) else { return };` without a denied
/// `assert!(false)` guard (the `assert!(cond); let else { return }` idiom).
pub(crate) fn require_button<M: Component>(app: &mut App) -> Option<Entity> {
    let found = single_with::<M>(app);
    assert!(
        found.is_some(),
        "exactly one button of the expected marker must be spawned in BattleRunning",
    );
    found
}

/// One fire-mode spec of an explicit [`ModeKind`] — the kind is what a toggle / the cycle
/// identifies a mode by (magnitudes arbitrary, not pinned tuning — the `acts.rs`
/// precedent).
pub(crate) const fn spec(kind: ModeKind, tu_percent: f32, shots: u16) -> FireModeSpec {
    FireModeSpec::new(
        kind,
        ModeConeMult::new(1.0),
        ModeTuPercent::new(tu_percent),
        ModeShots::new(shots),
    )
}

/// A three-mode `[Single, Burst, Full]` selector with three distinct modes.
pub(crate) fn sbf_selector() -> FireMode {
    FireMode::new(vec![
        spec(ModeKind::Single, 0.2, 1),
        spec(ModeKind::Burst, 0.4, 3),
        spec(ModeKind::Full, 0.7, 6),
    ])
}
