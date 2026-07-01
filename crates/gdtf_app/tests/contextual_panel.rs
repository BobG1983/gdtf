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
    AppState, BattleScapeState, ContextualPanelRoot, ExecuteButton, MeleeButton, OpenDoorButton,
    RunningState, ShoveButton, StabilizeButton,
};
use gdtf_battle_input::SelectedShooter;
use gdtf_battle_sim::{
    Cell, CellLevel, Direction, Facing, Faction, HeightBand, Level, LifeState, OpenState,
    OpenableBlocking, Position, Stabilized, Stance, StanceKind,
    acts::{ExecuteDownedRequested, MeleeRequested, MeleeTarget, ShoveRequested},
    entity::TerrainCell,
    injuries::InjuryRegistry,
    tuning::CombatTuning,
    weapon::WeaponRegistry,
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
    // GTW-505: the Load->Intro gate also requires a MeleeWeaponRegistry (empty-default
    // seed stands in for the asset-less resolve, mirroring the WeaponRegistry seed above).
    app.world_mut()
        .insert_resource(gdtf_battle_sim::weapon::MeleeWeaponRegistry::default());
    // GTW-269: the Load gate also requires an ArmorRegistry; empty clears it.
    app.world_mut()
        .insert_resource(gdtf_battle_sim::armor::ArmorRegistry::default());
    app.world_mut().insert_resource(InjuryRegistry::default());
    // GTW-415: the Load→Intro gate also requires a GangRegistry; empty clears it.
    app.world_mut()
        .insert_resource(gdtf_battle_sim::ganger::GangRegistry::default());
    // GTW-489: the NEW gate-blocking PrefabRegistry2; empty clears it.
    app.world_mut()
        .insert_resource(gdtf_battle_sim::level::PrefabRegistry2::default());
    // GTW-487: the NEW gate-blocking TerrainDefRegistry + UuidThemeRegistry.
    app.world_mut()
        .insert_resource(gdtf_battle_sim::terrain::def::TerrainDefRegistry::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::level::UuidThemeRegistry::default());

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
        single_with::<MeleeButton>(&mut app).is_some(),
        "the Melee button exists exactly once in BattleRunning (GTW-507)",
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
        visibility::<MeleeButton>(&mut app),
        Some(Visibility::Hidden),
        "the Melee button spawns Visibility::Hidden (scaffold: no detection yet — GTW-507)",
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
        single_with::<MeleeButton>(&mut app).is_none(),
        "the Melee button must be despawned with the panel (GTW-507)",
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

/// Spawns the MELEE actor — a ganger carrying exactly the components the melee detection reads
/// off the selection (its [`Position`] + [`Faction`] + [`Stance`] + [`Facing`], for the LOS
/// observer eye) — at cell `(x, y)` in gang `gang`, and SELECTS it (GTW-507). Returns its entity.
fn spawn_melee_actor(app: &mut App, x: i32, y: i32, gang: u8) -> Entity {
    let actor = app
        .world_mut()
        .spawn((
            at(x, y),
            Faction::new(gang),
            Stance::new(StanceKind::Standing),
            Facing::new(Direction::East),
        ))
        .id();
    app.world_mut().insert_resource(SelectedShooter::new(actor));
    actor
}

/// Spawns an ALIVE enemy at cell `(x, y)` in gang `gang` — [`LifeState::Alive`] plus its
/// [`Position`] / [`Faction`] / [`Stance`] (the melee detection scan + the LOS aim silhouette
/// read exactly these). Returns its entity (GTW-507).
fn spawn_alive_enemy(app: &mut App, x: i32, y: i32, gang: u8) -> Entity {
    app.world_mut()
        .spawn((
            at(x, y),
            Faction::new(gang),
            LifeState::Alive,
            Stance::new(StanceKind::Standing),
        ))
        .id()
}

/// Spawns the OPEN-DOOR actor — a ganger carrying exactly the components the open-door path reads:
/// its [`Position`] + [`Faction`] (detection adjacency) and a full [`Tu`] / [`TuMax`] pool (the
/// sim's `dispatch_open_door` affords + charges the `OpenDoorTu` leaf off it) — at cell `(x, y)` in
/// gang `gang`, and SELECTS it via [`SelectedShooter`] (GTW-315). Returns its entity. A generous TU
/// pool so the sim gate never rejects on affordability.
fn spawn_door_actor(app: &mut App, x: i32, y: i32, gang: u8) -> Entity {
    let actor = app
        .world_mut()
        .spawn((
            at(x, y),
            Faction::new(gang),
            gdtf_battle_sim::Tu::new(100),
            gdtf_battle_sim::TuMax::new(100),
        ))
        .id();
    app.world_mut().insert_resource(SelectedShooter::new(actor));
    actor
}

/// Spawns a CLOSED openable DOOR at cell `(x, y)` — a terrain entity carrying exactly what the
/// GTW-503 openable mechanism (and `setup_battle`) attach for an openable piece that the open-door
/// path reads/flips: its [`TerrainCell`] (the adjacency cell), [`OpenState::Closed`] (the scan
/// offers only a CLOSED door), and an [`OpenableBlocking`] band record (the toggle re-block source).
/// Returns its entity. The optional `state` overrides the closed default so a test can spawn an
/// already-OPEN door (which detection must NOT offer). The detection scan reads `OpenState` +
/// `TerrainCell`; `apply_openable_toggle` reads `OpenState` + `OpenableBlocking`.
fn spawn_door(app: &mut App, x: i32, y: i32, state: OpenState) -> Entity {
    app.world_mut()
        .spawn((
            TerrainCell::new(CellLevel::new(Cell::new(x, y), Level::new(0))),
            state,
            OpenableBlocking::new(HeightBand::High),
        ))
        .id()
}

/// Reads whether the Open Door button is visible (GTW-315).
fn open_door_visible(app: &mut App) -> bool {
    visibility::<OpenDoorButton>(app) == Some(Visibility::Visible)
}

/// Reads a door's current [`OpenState`], if it still carries one.
fn door_state(app: &App, door: Entity) -> Option<OpenState> {
    app.world().get::<OpenState>(door).copied()
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

/// Reads whether the Melee button is visible (GTW-507).
fn melee_visible(app: &mut App) -> bool {
    visibility::<MeleeButton>(app) == Some(Visibility::Visible)
}

/// Reads whether the Shove button is visible (GTW-525).
fn shove_visible(app: &mut App) -> bool {
    visibility::<ShoveButton>(app) == Some(Visibility::Visible)
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

/// Collected [`MeleeRequested`] messages (the GTW-507 press-test probe).
#[derive(Resource, Default)]
struct MeleeProbe(Vec<MeleeRequested>);

/// Adds the [`MeleeRequested`] probe, running AFTER the intent drain so it observes the SAME
/// update's emitted message (the `add_execute_probe` idiom — its own `MessageReader` cursor is
/// independent of the sim's `dispatch_melee`, so it reads every drained message even though the
/// sim consumes it too).
fn add_melee_probe(app: &mut App) {
    app.world_mut().insert_resource(MeleeProbe::default());
    app.add_systems(
        Update,
        (|mut r: MessageReader<MeleeRequested>, mut p: ResMut<MeleeProbe>| {
            p.0.extend(r.read().copied());
        })
        .after(gdtf_battle_input::dispatch_act_intents),
    );
}

/// The collected [`MeleeRequested`] messages.
fn melees(app: &App) -> Vec<MeleeRequested> {
    app.world()
        .get_resource::<MeleeProbe>()
        .map(|p| p.0.clone())
        .unwrap_or_default()
}

/// Collected [`ShoveRequested`] messages (the GTW-525 press-test probe).
#[derive(Resource, Default)]
struct ShoveProbe(Vec<ShoveRequested>);

/// Adds the [`ShoveRequested`] probe, running AFTER the intent drain so it observes the SAME
/// update's emitted message (the `add_melee_probe` idiom — its own `MessageReader` cursor is
/// independent of the sim's `dispatch_shove`, so it reads every drained message even though the
/// sim consumes it too).
fn add_shove_probe(app: &mut App) {
    app.world_mut().insert_resource(ShoveProbe::default());
    app.add_systems(
        Update,
        (|mut r: MessageReader<ShoveRequested>, mut p: ResMut<ShoveProbe>| {
            p.0.extend(r.read().copied());
        })
        .after(gdtf_battle_input::dispatch_act_intents),
    );
}

/// The collected [`ShoveRequested`] messages.
fn shoves(app: &App) -> Vec<ShoveRequested> {
    app.world()
        .get_resource::<ShoveProbe>()
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

// ---------------------------------------------------------------------------------
// GTW-507 — the MELEE button: detection (alive + 8-adjacent + LOS enemy reveals it) and
// press → MeleeRequested through the REAL seam.
// ---------------------------------------------------------------------------------

/// MELEE detection: a selected actor with an 8-adjacent, ALIVE, in-LOS ENEMY offers Melee — the
/// dedicated Melee button + the panel root become Visible (GTW-507). A STRONGER gate than
/// Execute's downed-adjacency: the enemy is ALIVE (not downed), and the LOS gate (reusing the
/// sim's `has_los` over the live battle grids) clears the open adjacent cell.
#[test]
fn adjacent_alive_enemy_in_los_offers_melee() {
    let mut app = battle_running_app();
    spawn_melee_actor(&mut app, 5, 5, 0);
    // An ALIVE ENEMY (gang 1) one cell diagonally — 8-adjacent, clear LOS (no cover between).
    spawn_alive_enemy(&mut app, 6, 6, 1);
    app.update();

    assert!(
        melee_visible(&mut app),
        "an 8-adjacent alive enemy in LOS must reveal the Melee button (GTW-507)",
    );
    assert!(
        root_visible(&mut app),
        "an offered Melee must reveal the panel root",
    );
    // The downed-only acts stay hidden — the enemy is ALIVE, not downed.
    assert!(
        !execute_visible(&mut app),
        "an ALIVE enemy is no Execute target (Execute needs a DOWNED enemy) -> hidden",
    );
    assert!(
        !stabilize_visible(&mut app),
        "no downed ally in reach -> the Stabilize button stays hidden",
    );
}

/// MELEE detection — the LIVE gate is stronger than mere adjacency: an alive enemy that is NOT
/// 8-adjacent does NOT offer Melee (the button stays hidden), and an alive ALLY (same faction)
/// never offers Melee. Discriminating: the same enemy moved INTO reach reveals it.
#[test]
fn non_adjacent_or_ally_does_not_offer_melee() {
    let mut app = battle_running_app();
    spawn_melee_actor(&mut app, 5, 5, 0);
    // An alive ENEMY far away (Chebyshev > 1) — not a melee candidate.
    let enemy = spawn_alive_enemy(&mut app, 20, 20, 1);
    // An alive ALLY 8-adjacent — same faction, never a melee target.
    spawn_alive_enemy(&mut app, 5, 6, 0);
    app.update();

    assert!(
        !melee_visible(&mut app),
        "a non-adjacent enemy + an adjacent ALLY offer NO melee (the button stays hidden)",
    );

    // Move the enemy INTO 8-adjacency — the SAME enemy now reveals the Melee button (the gate is
    // adjacency + alive + opposing + LOS, not mere presence).
    if let Some(mut pos) = app.world_mut().get_mut::<Position>(enemy) {
        *pos = at(6, 6);
    }
    app.update();
    assert!(
        melee_visible(&mut app),
        "moving the alive enemy into 8-adjacency reveals the Melee button (discriminating)",
    );
}

/// PRESS → INTENT: with a Melee target offered (an 8-adjacent alive in-LOS enemy), pressing the
/// Melee button drives the REAL stack (button -> `ActIntent::Melee(target)` -> the ONE
/// `dispatch_act_intents` drain) to emit exactly one `MeleeRequested` for the `SelectedShooter`
/// as attacker over the carried target (GTW-507) — driven THROUGH the button/intent path, NOT a
/// synthetic `MeleeRequested` emit.
///
/// Pin-discriminating: dropping the Melee arm in `contextual_button_intents` (or the detection
/// that fills the target) leaves the queue empty and emits zero messages, failing the asserts.
#[test]
fn pressing_melee_emits_melee_requested_for_target() {
    let mut app = battle_running_app();
    add_melee_probe(&mut app);
    let attacker = spawn_melee_actor(&mut app, 5, 5, 0);
    let target = spawn_alive_enemy(&mut app, 6, 6, 1);

    // First update: detection reveals the Melee button + fills the target offer.
    app.update();
    assert!(
        melee_visible(&mut app),
        "sanity: the Melee button is offered before the press",
    );
    let Some(melee_btn) = single_with::<MeleeButton>(&mut app) else {
        // The button must exist by construction; bail without a panic (restriction lints deny
        // `panic!` even in tests). A missing button trips the `melee_visible` assert above.
        return;
    };

    // Drive the press, then update: contextual_button_intents pushes Melee(target) and the ONE
    // drain (ordered after it) emits MeleeRequested the SAME update.
    press_button(&mut app, melee_btn);
    app.update();

    let emitted = melees(&app);
    assert_eq!(
        emitted.len(),
        1,
        "pressing Melee with a target offered must emit exactly one MeleeRequested",
    );
    assert_eq!(
        emitted[0].attacker, attacker,
        "the attacker is the SelectedShooter",
    );
    assert_eq!(
        emitted[0].target,
        MeleeTarget::Ganger(target),
        "the target is the carried opposing neighbour (the ganger melee form)",
    );
}

// ---------------------------------------------------------------------------------
// GTW-525 — the SHOVE button: detection (an 8-adjacent alive opposing ganger reveals it,
// WEAKER than Melee — no LOS / no weapon) and press → ShoveRequested through the REAL seam.
// ---------------------------------------------------------------------------------

/// SHOVE detection: a selected actor (any ganger — the plain `spawn_actor`, which carries NO
/// stance / facing, so NO melee is ever offered) with an 8-adjacent, ALIVE, OPPOSING ganger
/// offers Shove — the dedicated Shove button + the panel root become Visible (GTW-525). The gate
/// is WEAKER than Melee's: NO LOS and NO weapon are needed (any ganger can shove any alive
/// opposing neighbour), which is why the LOS-less `spawn_actor` still reveals it.
#[test]
fn adjacent_alive_opposing_offers_shove() {
    let mut app = battle_running_app();
    spawn_actor(&mut app, 5, 5, 0);
    // An ALIVE ENEMY (gang 1) one cell diagonally — 8-adjacent.
    spawn_alive_enemy(&mut app, 6, 6, 1);
    app.update();

    assert!(
        shove_visible(&mut app),
        "an 8-adjacent alive opposing ganger must reveal the Shove button (GTW-525)",
    );
    assert!(
        root_visible(&mut app),
        "an offered Shove must reveal the panel root",
    );
    // Melee needs the actor's stance + facing (the LOS eye); a plain actor carries neither, so
    // Melee is NOT offered even though an alive enemy is adjacent — Shove is the WEAKER gate.
    assert!(
        !melee_visible(&mut app),
        "a stance/facing-less actor offers NO melee, yet Shove still reveals (weaker gate)",
    );
    // The downed-only acts stay hidden — the enemy is ALIVE, not downed.
    assert!(
        !execute_visible(&mut app),
        "an ALIVE enemy is no Execute target (Execute needs a DOWNED enemy) -> hidden",
    );
    assert!(
        !stabilize_visible(&mut app),
        "no downed ally in reach -> the Stabilize button stays hidden",
    );
}

/// SHOVE detection — the gate is adjacency + alive + opposing (NO downed / NO ally): a
/// non-adjacent alive enemy, an adjacent ALLY, and an adjacent DOWNED enemy each offer NO shove
/// (the button stays hidden). Discriminating: moving the alive enemy INTO 8-adjacency reveals it.
#[test]
fn non_adjacent_ally_or_downed_does_not_offer_shove() {
    let mut app = battle_running_app();
    spawn_actor(&mut app, 5, 5, 0);
    // An alive ENEMY far away (Chebyshev > 1) — not a shove candidate.
    let enemy = spawn_alive_enemy(&mut app, 20, 20, 1);
    // An alive ALLY 8-adjacent — same faction, never a shove target.
    spawn_alive_enemy(&mut app, 5, 6, 0);
    // A DOWNED enemy 8-adjacent — not ALIVE, so no shove (the deliberate gate needs `is_active`).
    spawn_downed(&mut app, 4, 4, 1, None);
    app.update();

    assert!(
        !shove_visible(&mut app),
        "a non-adjacent enemy + an adjacent ally + an adjacent DOWNED enemy offer NO shove",
    );

    // Move the alive enemy INTO 8-adjacency — the SAME enemy now reveals the Shove button (the
    // gate is adjacency + alive + opposing, not mere presence).
    if let Some(mut pos) = app.world_mut().get_mut::<Position>(enemy) {
        *pos = at(6, 6);
    }
    app.update();
    assert!(
        shove_visible(&mut app),
        "moving the alive opposing enemy into 8-adjacency reveals the Shove button (discriminating)",
    );
}

/// PRESS → INTENT: with a Shove target offered (an 8-adjacent alive opposing ganger), pressing the
/// Shove button drives the REAL stack (button -> `ActIntent::Shove(target)` -> the ONE
/// `dispatch_act_intents` drain) to emit exactly one `ShoveRequested` for the `SelectedShooter`
/// as shover over the carried target (GTW-525) — driven THROUGH the button/intent path, NOT a
/// synthetic `ShoveRequested` emit. The emitted request is the DELIBERATE form
/// (`ShoveSource::Deliberate`).
///
/// Pin-discriminating: dropping the Shove arm in `contextual_button_intents` (or the detection
/// that fills the target) leaves the queue empty and emits zero messages, failing the asserts.
#[test]
fn pressing_shove_emits_shove_requested_for_target() {
    let mut app = battle_running_app();
    add_shove_probe(&mut app);
    let shover = spawn_actor(&mut app, 5, 5, 0);
    let target = spawn_alive_enemy(&mut app, 6, 6, 1);

    // First update: detection reveals the Shove button + fills the target offer.
    app.update();
    assert!(
        shove_visible(&mut app),
        "sanity: the Shove button is offered before the press",
    );
    let Some(shove_btn) = single_with::<ShoveButton>(&mut app) else {
        // The button must exist by construction; bail without a panic (restriction lints deny
        // `panic!` even in tests). A missing button trips the `shove_visible` assert above.
        return;
    };

    // Drive the press, then update: contextual_button_intents pushes Shove(target) and the ONE
    // drain (ordered after it) emits ShoveRequested the SAME update.
    press_button(&mut app, shove_btn);
    app.update();

    let emitted = shoves(&app);
    assert_eq!(
        emitted.len(),
        1,
        "pressing Shove with a target offered must emit exactly one ShoveRequested",
    );
    assert_eq!(
        emitted[0].shover, shover,
        "the shover is the SelectedShooter",
    );
    assert_eq!(
        emitted[0].target, target,
        "the target is the carried opposing neighbour",
    );
}

// ---------------------------------------------------------------------------------
// GTW-315 — the OPEN DOOR button: detection (an 8-adjacent CLOSED door reveals it, an
// already-open / non-adjacent door does NOT) and press → the door's OpenState toggles
// to Open through the REAL seam + the app-wired sim.
// ---------------------------------------------------------------------------------

/// OPEN-DOOR detection: a selected player actor with an 8-adjacent CLOSED door offers Open Door —
/// the dedicated Open Door button + the panel root become Visible (GTW-315). The button ALWAYS
/// OPENS (a closed door is the only offered target); F4 is PLAYER-ONLY (the selected actor is the
/// player-faction ganger). No ganger acts are offered (no downed / alive neighbour present).
#[test]
fn adjacent_closed_door_offers_open_door() {
    let mut app = battle_running_app();
    spawn_door_actor(&mut app, 5, 5, 0);
    // A CLOSED door one cell diagonally — 8-adjacent.
    spawn_door(&mut app, 6, 6, OpenState::Closed);
    app.update();

    assert!(
        open_door_visible(&mut app),
        "an 8-adjacent CLOSED door must reveal the Open Door button (GTW-315)",
    );
    assert!(
        root_visible(&mut app),
        "an offered Open Door must reveal the panel root",
    );
    // No ganger neighbour present -> the ganger acts stay hidden.
    assert!(
        !execute_visible(&mut app),
        "no downed enemy in reach -> the Execute button stays hidden",
    );
    assert!(
        !stabilize_visible(&mut app),
        "no downed ally in reach -> the Stabilize button stays hidden",
    );
    assert!(
        !melee_visible(&mut app),
        "no meleeable neighbour in reach -> the Melee button stays hidden",
    );
    assert!(
        !shove_visible(&mut app),
        "no shovable neighbour in reach -> the Shove button stays hidden",
    );
}

/// OPEN-DOOR detection — the gate is CLOSED + 8-adjacent: an already-OPEN adjacent door and a
/// CLOSED but non-adjacent door each offer NO open (the button + panel root stay hidden, and no
/// stray ganger act is offered). Discriminating: moving the actor next to the CLOSED far door
/// reveals it.
#[test]
fn open_or_non_adjacent_door_does_not_offer_open_door() {
    let mut app = battle_running_app();
    let actor = spawn_door_actor(&mut app, 5, 5, 0);
    // An already-OPEN door 8-adjacent — the button only OPENS, so it is NOT offered.
    spawn_door(&mut app, 5, 6, OpenState::Open);
    // A CLOSED door FAR away (Chebyshev > 1) — not adjacent, so not offered.
    spawn_door(&mut app, 30, 30, OpenState::Closed);
    app.update();

    assert!(
        !open_door_visible(&mut app),
        "an adjacent OPEN door + a non-adjacent CLOSED door offer NO open (button hidden)",
    );
    assert!(
        !root_visible(&mut app),
        "with no contextual act offered the panel root stays hidden",
    );

    // Move the actor next to the FAR closed door — the CLOSED + 8-adjacent gate now passes and the
    // Open Door button reveals (discriminating: the gate is CLOSED + adjacency, not mere presence).
    if let Some(mut pos) = app.world_mut().get_mut::<Position>(actor) {
        *pos = at(29, 30);
    }
    app.update();
    assert!(
        open_door_visible(&mut app),
        "moving the actor next to the CLOSED door reveals the Open Door button (discriminating)",
    );
}

/// PRESS → OPEN: with an Open Door target offered (an 8-adjacent CLOSED door), pressing the Open
/// Door button drives the REAL stack (button -> `ActIntent::OpenDoor(door)` -> the ONE
/// `dispatch_act_intents` drain -> `OpenDoorRequested` -> the app-wired sim `dispatch_open_door` ->
/// `SetOpenable::toggle` -> `apply_openable_toggle`) so the SPECIFIC carried door's `OpenState`
/// flips CLOSED -> Open (GTW-315). Driven THROUGH the button/intent/sim path end to end — never a
/// synthetic `SetOpenable` emit — proving the correct door entity was carried across the seam.
///
/// The door flip settles one frame after the toggle message (the GTW-503 documented one-frame
/// settle: `dispatch_open_door` writes `SetOpenable`, `apply_openable_toggle` flips `OpenState` and
/// removes the blocking components), so the assertion advances a couple of updates.
///
/// Pin-discriminating: dropping the Open Door arm in `contextual_button_intents` (or the detection
/// that fills the target) leaves the queue empty, no `OpenDoorRequested` is emitted, and the door
/// stays CLOSED, failing the assert.
#[test]
fn pressing_open_door_toggles_the_door_open() {
    let mut app = battle_running_app();
    spawn_door_actor(&mut app, 5, 5, 0);
    let door = spawn_door(&mut app, 6, 6, OpenState::Closed);

    // First update: detection reveals the Open Door button + fills the door offer.
    app.update();
    assert!(
        open_door_visible(&mut app),
        "sanity: the Open Door button is offered before the press",
    );
    assert_eq!(
        door_state(&app, door),
        Some(OpenState::Closed),
        "sanity: the door is CLOSED before the press",
    );
    let Some(open_door_btn) = single_with::<OpenDoorButton>(&mut app) else {
        // The button must exist by construction; bail without a panic (restriction lints deny
        // `panic!` even in tests). A missing button trips the `open_door_visible` assert above.
        return;
    };

    // Drive the press, then advance: contextual_button_intents pushes OpenDoor(door), the drain
    // emits OpenDoorRequested + the sim dispatch writes SetOpenable::toggle the SAME update, and
    // apply_openable_toggle flips OpenState one frame later (the GTW-503 one-frame settle).
    press_button(&mut app, open_door_btn);
    let opened = advance_until(
        &mut app,
        |app| door_state(app, door) == Some(OpenState::Open),
        BUDGET,
    );
    assert!(
        opened,
        "pressing Open Door on the carried CLOSED door must toggle its OpenState to Open through \
         the real seam + sim; last was {:?}",
        door_state(&app, door),
    );
}
