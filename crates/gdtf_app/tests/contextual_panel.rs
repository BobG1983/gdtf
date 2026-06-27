//! GTW-294 — the battlescape CONTEXTUAL PANEL (bottom-right), driven through the REAL app stack.
//!
//! These headless `GdtfTestAppBuilder` integration tests drive the genuine state machine down to
//! `BattleScapeState::BattleRunning`, where the real contextual-panel plugin spawns its tree on
//! `OnEnter`, tears it down on `OnExit`, and runs its live detection + press-routing systems.
//! They cover the contract:
//!
//! - **Scaffold** — on entering `BattleRunning` the bare `ContextualPanelRoot` box and all three
//!   buttons (Execute / Stabilize / Open Door) exist; the box + buttons spawn `Visibility::Hidden`;
//!   the box carries a `GlobalZIndex` above the bottom bar so it draws on top of it (the GTW-294
//!   occlusion fix); on exiting `BattleRunning` the whole subtree (box → buttons) is despawned
//!   (battle-scoped lifecycle).
//! - **Detection** — a selected actor with an 8-adjacent downed ENEMY offers Execute (button +
//!   panel visible, target set); with an 8-adjacent unstabilized downed ALLY offers Stabilize;
//!   with no adjacent downed neighbour the panel + all buttons hide and both targets clear.
//! - **Reactive, no respawn** — moving the actor away (or clearing selection) hides the panel
//!   while the SAME button entities persist (a `Visibility` toggle, never a despawn/respawn).
//! - **Press → intent** — with an Execute target offered, pressing the Execute button drives
//!   the REAL seam (button → 222a intent → the ONE drain) to emit one `ExecuteDownedRequested`
//!   for the selection as actor over the carried downed target.

use bevy::{ecs::entity::Entity, prelude::*, state::state::State};
use gdtf_app::test_support::{
    AppState, BattleScapeState, ContextualPanelRoot, ExecuteButton, OpenDoorButton, RunningState,
    StabilizeButton,
};
use gdtf_battle_input::SelectedShooter;
use gdtf_battle_sim::{
    Cell, CellLevel, Faction, Level, LifeState, Position, Stabilized, acts::ExecuteDownedRequested,
    injuries::InjuryRegistry, level::ThemeCatalogRegistry, terrain::piece::TerrainRegistry,
    tuning::CombatTuning, weapon::WeaponRegistry,
};
use gdtf_test_utils::{GdtfTestAppBuilder, advance_until};
use gdtf_ui::theme::default_theme;

/// A budget large enough to drive the deep walk into the battlescape, bounded so a machine that
/// never reaches the predicate fails instead of hanging.
const BUDGET: u32 = 96;

/// Reads the current [`BattleScapeState`] if active.
fn battlescape_state(app: &App) -> Option<BattleScapeState> {
    app.world()
        .get_resource::<State<BattleScapeState>>()
        .map(|state| *state.get())
}

/// Reads the current [`RunningState`] if active.
fn running_state(app: &App) -> Option<RunningState> {
    app.world()
        .get_resource::<State<RunningState>>()
        .map(|state| *state.get())
}

/// Drives the real stack to `BattleScapeState::BattleRunning`, where the contextual panel is live.
fn battle_running_app() -> App {
    let mut app = GdtfTestAppBuilder::new_with_scene_support()
        .starting_in(AppState::Running)
        .build();
    app.world_mut().insert_resource(default_theme());
    app.world_mut().insert_resource(CombatTuning::default());
    app.world_mut().insert_resource(WeaponRegistry::default());
    // GTW-269: the Load gate also requires an ArmorRegistry; empty clears it.
    app.world_mut()
        .insert_resource(gdtf_battle_sim::armor::ArmorRegistry::default());
    // GTW-394: the Load gate also requires a TerrainRegistry; empty clears it.
    app.world_mut().insert_resource(TerrainRegistry::default());
    app.world_mut()
        .insert_resource(ThemeCatalogRegistry::default());
    app.world_mut().insert_resource(InjuryRegistry::default());
    // GTW-415: the Load→Intro gate also requires a GangRegistry; empty clears it.
    app.world_mut()
        .insert_resource(gdtf_battle_sim::ganger::GangRegistry::default());
    // GTW-418: the Load gate also requires a PrefabRegistry; empty clears it.
    app.world_mut()
        .insert_resource(gdtf_battle_sim::level::PrefabRegistry::default());

    let at_menu = advance_until(
        &mut app,
        |app| running_state(app) == Some(RunningState::Menu),
        BUDGET,
    );
    assert!(at_menu, "the walk should reach RunningState::Menu");
    app.world_mut()
        .resource_mut::<NextState<RunningState>>()
        .set(RunningState::Game);
    let at_battle = advance_until(
        &mut app,
        |app| battlescape_state(app) == Some(BattleScapeState::BattleRunning),
        BUDGET,
    );
    assert!(
        at_battle,
        "the walk should reach BattleScapeState::BattleRunning; last was {:?}",
        battlescape_state(&app),
    );
    app
}

/// All entities carrying marker `M`.
fn all_with<M: Component>(app: &mut App) -> Vec<Entity> {
    let mut q = app.world_mut().query_filtered::<Entity, With<M>>();
    q.iter(app.world()).collect()
}

/// The single entity carrying marker `M`, or `None` if not exactly one.
fn single_with<M: Component>(app: &mut App) -> Option<Entity> {
    match all_with::<M>(app).as_slice() {
        [one] => Some(*one),
        _ => None,
    }
}

/// The [`Visibility`] of the single entity carrying marker `M`.
fn visibility<M: Component>(app: &mut App) -> Option<Visibility> {
    let entity = single_with::<M>(app)?;
    app.world().get::<Visibility>(entity).copied()
}

/// The parent [`Entity`] of `child` via its [`ChildOf`] relation, or `None` if it has no parent.
fn parent_of(app: &App, child: Entity) -> Option<Entity> {
    app.world().get::<ChildOf>(child).map(ChildOf::parent)
}

// ---------------------------------------------------------------------------------
// Scaffold AC — the panel root + all three buttons spawn in BattleRunning, each with
// its marker and `Visibility::Hidden`; they despawn outside BattleRunning.
// ---------------------------------------------------------------------------------

/// In the live battle the contextual panel has spawned the bare panel-box root + exactly one
/// button per contextual act (Execute / Stabilize / Open Door). The box + buttons spawn
/// `Visibility::Hidden` (no detection reveals them in this state). The root is a BARE top-level
/// node (no wrapper parent) carrying a `GlobalZIndex` above the bottom bar so it draws on top of
/// it (the GTW-294 occlusion fix).
#[test]
fn contextual_panel_spawns_hidden_in_battle() {
    let mut app = battle_running_app();

    let root = single_with::<ContextualPanelRoot>(&mut app);
    assert!(
        root.is_some(),
        "exactly one contextual panel root is spawned in BattleRunning",
    );
    assert!(
        single_with::<ExecuteButton>(&mut app).is_some(),
        "the Execute button exists exactly once in BattleRunning",
    );
    assert!(
        single_with::<StabilizeButton>(&mut app).is_some(),
        "the Stabilize button exists exactly once in BattleRunning",
    );
    assert!(
        single_with::<OpenDoorButton>(&mut app).is_some(),
        "the Open Door button exists exactly once in BattleRunning",
    );

    // The panel-box root is a BARE top-level node (the bottom-bar precedent — no wrapper parent),
    // carrying a `GlobalZIndex` strictly above the bottom bar so it draws ON TOP of the opaque bar
    // it overlaps (the GTW-294 occlusion fix). The `is_some` assert above already failed loudly if
    // the root is missing; bind without a panic (restriction lints deny `panic!` even in tests).
    let Some(root) = root else {
        return;
    };
    assert_eq!(
        parent_of(&app, root),
        None,
        "the contextual panel root must be a bare top-level node (no wrapper parent)",
    );
    let z = app.world().get::<GlobalZIndex>(root);
    assert!(
        z.is_some_and(|z| z.0 > 10),
        "the contextual panel root must carry a GlobalZIndex above the bottom bar (10) so the \
         opaque bar does not paint over it; was {z:?}",
    );

    // SCAFFOLD: the panel box AND every button are hidden by default — no detection reveals
    // them in this state.
    assert_eq!(
        visibility::<ContextualPanelRoot>(&mut app),
        Some(Visibility::Hidden),
        "the contextual panel root spawns Visibility::Hidden (scaffold: no detection yet)",
    );
    assert_eq!(
        visibility::<ExecuteButton>(&mut app),
        Some(Visibility::Hidden),
        "the Execute button spawns Visibility::Hidden (scaffold: no detection yet)",
    );
    assert_eq!(
        visibility::<StabilizeButton>(&mut app),
        Some(Visibility::Hidden),
        "the Stabilize button spawns Visibility::Hidden (scaffold: no detection yet)",
    );
    assert_eq!(
        visibility::<OpenDoorButton>(&mut app),
        Some(Visibility::Hidden),
        "the Open Door button spawns Visibility::Hidden (deferred act)",
    );
}

/// Once the battle leaves `BattleRunning` the contextual panel subtree — the panel box and its
/// three buttons — is despawned (battle-scoped `OnExit` cleanup over the ROOT marker, mirroring
/// the action bar / bottom bar).
#[test]
fn contextual_panel_despawns_outside_battle() {
    let mut app = battle_running_app();
    assert!(
        single_with::<ContextualPanelRoot>(&mut app).is_some(),
        "sanity: the contextual panel box is present in BattleRunning before we leave it",
    );

    // Leave BattleRunning: the battlescape PERSISTS (GTW-236), so the test inserts the explicit
    // `BattleRunningComplete` end-signal marker to trip `move_on` and advance the machine out of
    // BattleRunning, where `OnExit` despawns the panel. The marker is reached through the same
    // `test_support` surface the action-bar / weapon-panel tests use.
    app.world_mut()
        .insert_resource(gdtf_app::test_support::BattleRunningComplete);
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
        single_with::<ContextualPanelRoot>(&mut app).is_none(),
        "the contextual panel box must be despawned once the battle leaves BattleRunning",
    );
    assert!(
        single_with::<ExecuteButton>(&mut app).is_none(),
        "the Execute button must be despawned with the panel",
    );
    assert!(
        single_with::<StabilizeButton>(&mut app).is_none(),
        "the Stabilize button must be despawned with the panel",
    );
    assert!(
        single_with::<OpenDoorButton>(&mut app).is_none(),
        "the Open Door button must be despawned with the panel",
    );
}

// ---------------------------------------------------------------------------------
// Live slice helpers — spawn an actor + downed neighbours, select, read targets, and
// drive a press through the real seam.
// ---------------------------------------------------------------------------------

/// A ground-level [`Position`] at cell `(x, y)`.
fn at(x: i32, y: i32) -> Position {
    Position::new(CellLevel::new(Cell::new(x, y), Level::new(0)))
}

/// Spawns the ACTOR — a ganger carrying exactly the components detection reads off the selection
/// (its [`Position`] + [`Faction`]) — at cell `(x, y)` in gang `gang`, and SELECTS it via the
/// [`SelectedShooter`] resource (the selection detection + the press router read). Returns its
/// entity.
fn spawn_actor(app: &mut App, x: i32, y: i32, gang: u8) -> Entity {
    let actor = app.world_mut().spawn((at(x, y), Faction::new(gang))).id();
    app.world_mut().insert_resource(SelectedShooter::new(actor));
    actor
}

/// Spawns a DOWNED neighbour at cell `(x, y)` in gang `gang` — [`LifeState::Downed`] plus its
/// [`Position`] / [`Faction`], and (when `stabilized` is `Some`) a [`Stabilized`] flag. Returns
/// its entity. The detection scan reads exactly these components.
fn spawn_downed(app: &mut App, x: i32, y: i32, gang: u8, stabilized: Option<bool>) -> Entity {
    let mut entity = app
        .world_mut()
        .spawn((at(x, y), Faction::new(gang), LifeState::Downed));
    if let Some(flag) = stabilized {
        entity.insert(Stabilized::new(flag));
    }
    entity.id()
}

/// The current [`ContextualTargets`] offers via the END message they route to is not directly
/// readable across the crate boundary (the seam's contents are private), so detection coverage
/// reads the panel's observable effects — the per-button [`Visibility`] — and the press test
/// reads the emitted [`ExecuteDownedRequested`]. This helper reads the Execute button visibility.
fn execute_visible(app: &mut App) -> bool {
    visibility::<ExecuteButton>(app) == Some(Visibility::Visible)
}

/// Reads whether the Stabilize button is visible.
fn stabilize_visible(app: &mut App) -> bool {
    visibility::<StabilizeButton>(app) == Some(Visibility::Visible)
}

/// Reads whether the panel root is visible.
fn root_visible(app: &mut App) -> bool {
    visibility::<ContextualPanelRoot>(app) == Some(Visibility::Visible)
}

/// Collected [`ExecuteDownedRequested`] messages (the press-test probe).
#[derive(Resource, Default)]
struct ExecuteProbe(Vec<ExecuteDownedRequested>);

/// Adds the [`ExecuteDownedRequested`] probe, running AFTER the intent drain so it observes the
/// SAME update's emitted message (the `action_bar.rs` `add_probes` idiom — its own
/// `MessageReader` cursor is independent of the sim's dispatch, so it reads every drained
/// message).
fn add_execute_probe(app: &mut App) {
    app.world_mut().insert_resource(ExecuteProbe::default());
    app.add_systems(
        Update,
        (|mut r: MessageReader<ExecuteDownedRequested>, mut p: ResMut<ExecuteProbe>| {
            p.0.extend(r.read().copied());
        })
        .after(gdtf_battle_input::dispatch_act_intents),
    );
}

/// The collected [`ExecuteDownedRequested`] messages.
fn executes(app: &App) -> Vec<ExecuteDownedRequested> {
    app.world()
        .get_resource::<ExecuteProbe>()
        .map(|p| p.0.clone())
        .unwrap_or_default()
}

/// Synthesizes a fresh mouse press on `button` (the `action_bar.rs` `press_button` idiom): a
/// direct write of [`Interaction::Pressed`] marks the component `Changed` this update, so the
/// `Changed<Interaction>` press query fires.
fn press_button(app: &mut App, button: Entity) {
    if let Some(mut interaction) = app.world_mut().get_mut::<Interaction>(button) {
        *interaction = Interaction::Pressed;
    }
}

// ---------------------------------------------------------------------------------
// Detection AC — a selected actor's actionable downed neighbours drive the panel's
// reactive show/hide.
// ---------------------------------------------------------------------------------

/// EXECUTE condition: a selected actor with an 8-adjacent downed ENEMY (different faction) offers
/// Execute — the Execute button + the panel root become Visible while the Stabilize button stays
/// Hidden (no downed ALLY in reach).
#[test]
fn adjacent_downed_enemy_offers_execute() {
    let mut app = battle_running_app();
    spawn_actor(&mut app, 5, 5, 0);
    // A downed ENEMY (gang 1) one cell diagonally — 8-adjacent.
    spawn_downed(&mut app, 6, 6, 1, None);
    app.update();

    assert!(
        execute_visible(&mut app),
        "an 8-adjacent downed enemy must reveal the Execute button",
    );
    assert!(
        root_visible(&mut app),
        "an offered Execute must reveal the panel root",
    );
    assert!(
        !stabilize_visible(&mut app),
        "no downed ally in reach -> the Stabilize button stays hidden",
    );
}

/// STABILIZE condition: a selected actor with an 8-adjacent unstabilized downed ALLY (same
/// faction) offers Stabilize — the Stabilize button + the panel root become Visible.
#[test]
fn adjacent_downed_ally_offers_stabilize() {
    let mut app = battle_running_app();
    spawn_actor(&mut app, 5, 5, 0);
    // A downed, not-yet-stabilized ALLY (gang 0) orthogonally adjacent — 8-adjacent.
    spawn_downed(&mut app, 5, 6, 0, Some(false));
    app.update();

    assert!(
        stabilize_visible(&mut app),
        "an 8-adjacent unstabilized downed ally must reveal the Stabilize button",
    );
    assert!(
        root_visible(&mut app),
        "an offered Stabilize must reveal the panel root",
    );
    assert!(
        !execute_visible(&mut app),
        "no downed enemy in reach -> the Execute button stays hidden",
    );
}

/// HIDDEN condition: a selected actor with NO adjacent downed neighbour hides the panel root +
/// all three buttons (no act offered).
#[test]
fn no_adjacent_downed_hides_panel() {
    let mut app = battle_running_app();
    spawn_actor(&mut app, 5, 5, 0);
    // A downed enemy FAR away (not 8-adjacent) — not a candidate.
    spawn_downed(&mut app, 20, 20, 1, None);
    app.update();

    assert!(
        !root_visible(&mut app),
        "no downed neighbour in reach -> the panel root stays hidden",
    );
    assert!(
        !execute_visible(&mut app),
        "no downed enemy in reach -> the Execute button stays hidden",
    );
    assert!(
        !stabilize_visible(&mut app),
        "no downed ally in reach -> the Stabilize button stays hidden",
    );
    assert_eq!(
        visibility::<OpenDoorButton>(&mut app),
        Some(Visibility::Hidden),
        "the Open Door button is a deferred act and stays hidden",
    );
}

/// REACTIVE / no respawn: after the panel shows for an adjacent downed enemy, moving the actor
/// out of reach hides it again — and the SAME button entities persist (asserted by `Entity` id),
/// proving the system TOGGLES `Visibility` rather than despawning + respawning the scaffold.
#[test]
fn moving_actor_away_hides_panel_without_respawn() {
    let mut app = battle_running_app();
    let actor = spawn_actor(&mut app, 5, 5, 0);
    spawn_downed(&mut app, 6, 6, 1, None);
    app.update();

    // Sanity: the panel is showing, and capture the button entity ids.
    assert!(
        execute_visible(&mut app),
        "sanity: the Execute button is shown"
    );
    let execute_before = single_with::<ExecuteButton>(&mut app);
    let stabilize_before = single_with::<StabilizeButton>(&mut app);
    let root_before = single_with::<ContextualPanelRoot>(&mut app);

    // Move the actor far from the downed enemy (mutate Position in place).
    if let Some(mut pos) = app.world_mut().get_mut::<Position>(actor) {
        *pos = at(40, 40);
    }
    app.update();

    assert!(
        !root_visible(&mut app),
        "moving the actor out of reach must hide the panel root",
    );
    assert!(
        !execute_visible(&mut app),
        "moving the actor out of reach must hide the Execute button",
    );

    // The SAME entities still exist — a Visibility toggle, not a despawn/respawn.
    assert_eq!(
        single_with::<ExecuteButton>(&mut app),
        execute_before,
        "the Execute button entity must persist (Visibility toggle, not respawn)",
    );
    assert_eq!(
        single_with::<StabilizeButton>(&mut app),
        stabilize_before,
        "the Stabilize button entity must persist (Visibility toggle, not respawn)",
    );
    assert_eq!(
        single_with::<ContextualPanelRoot>(&mut app),
        root_before,
        "the panel root entity must persist (Visibility toggle, not respawn)",
    );
}

// ---------------------------------------------------------------------------------
// Press → intent AC — a contextual button press routes the carried target through the
// REAL 222a seam (button -> intent -> the ONE drain).
// ---------------------------------------------------------------------------------

/// PRESS → INTENT: with an Execute target offered (an 8-adjacent downed enemy), pressing the
/// Execute button drives the REAL stack (button -> `ActIntent::Execute(target)` -> the ONE
/// `dispatch_act_intents` drain) to emit exactly one `ExecuteDownedRequested` for the
/// `SelectedShooter` as actor over the carried downed target. The shared `PendingActIntent` seam
/// is private (contents unreadable across the crate boundary) and the drain empties it each
/// update, so the load-bearing assertion is the END message the press produces — the same parity
/// idiom the action-bar end-turn test uses, and strictly stronger than reading the queue.
///
/// Pin-discriminating: dropping the Execute arm in `contextual_button_intents` (or the detection
/// that fills the target) leaves the queue empty and emits zero messages, failing the asserts.
#[test]
fn pressing_execute_emits_execute_downed_requested_for_target() {
    let mut app = battle_running_app();
    add_execute_probe(&mut app);
    let actor = spawn_actor(&mut app, 5, 5, 0);
    let target = spawn_downed(&mut app, 6, 6, 1, None);

    // First update: detection reveals the Execute button + fills the target offer.
    app.update();
    assert!(
        execute_visible(&mut app),
        "sanity: the Execute button is offered before the press",
    );
    let Some(execute_btn) = single_with::<ExecuteButton>(&mut app) else {
        // The button must exist by construction; bail without a panic (restriction lints deny
        // `panic!` even in tests). A missing button trips the `execute_visible` assert above.
        return;
    };

    // Drive the press, then update: contextual_button_intents pushes Execute(target) and the ONE
    // drain (ordered after it) emits ExecuteDownedRequested the SAME update.
    press_button(&mut app, execute_btn);
    app.update();

    let emitted = executes(&app);
    assert_eq!(
        emitted.len(),
        1,
        "pressing Execute with a target offered must emit exactly one ExecuteDownedRequested",
    );
    assert_eq!(emitted[0].actor, actor, "the actor is the SelectedShooter");
    assert_eq!(
        emitted[0].target, target,
        "the target is the carried downed neighbour",
    );
}
