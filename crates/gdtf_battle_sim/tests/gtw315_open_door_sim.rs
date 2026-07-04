//! GTW-315 — the LIVE OPEN-DOOR act, proven END-TO-END through the REAL wiring
//! (`dispatch_open_door` + the GTW-503 `apply_openable_toggle` mechanism + the GTW-501 / GTW-502
//! path/vision projection).
//!
//! The clause contract:
//!
//! - **(a) valid request charges exactly `OpenDoorTu` + toggles CLOSED → open** — a deliberate
//!   open-door request from an 8-adjacent actor with enough TU flips the door to `OpenState::Open`
//!   and drops the actor's TU by EXACTLY the shipped `open_door_tu` leaf.
//! - **(b) re-gate rejects** — a NON-ADJACENT actor, an actor with INSUFFICIENT TU, and an
//!   ALREADY-OPEN door each leave the door state untouched AND spend NO TU (no toggle, no charge).
//! - **(c) after opening, the door no longer blocks pathing/vision** — asserted through the
//!   EXISTING GTW-503 downstream (`apply_openable_toggle` → the GTW-501/502 change-detection):
//!   a CLOSED door blocks path + occludes vision; opening it via the act clears BOTH.
//!
//! The TU-charge assert pins the DROP to the shipped `open_door_tu` leaf (the act's defining
//! behaviour — "costs exactly the `OpenDoorTu` leaf"), reading the leaf from the tuning resource
//! rather than hard-coding a magnitude, so it survives a re-tune (the drop tracks the leaf).
//! Render-free, zero pixels; the one `World` mutation is in a TEST BODY (`bevy-traps.md` #7
//! carve-out).

use bevy::{
    app::App,
    prelude::{Entity, World},
};
use gdtf_battle_sim::{
    Cell, CellLevel, CombatTuning, Faction, HeightBand, Level, OccupancyGrid,
    OccupancyMaintenancePlugin, OpenDoorRequested, OpenState, OpenableBlocking,
    OpenableTogglePlugin, Position, Tu, TuMax,
    entity::{BlocksPathfinding, BlocksVision, TerrainCell},
    test_support::SimAppBuilder,
};

/// An arbitrary fixed seed — the open-door act is RNG-free, so the value is irrelevant; it only
/// seeds the sibling `Simulate`-band RNG streams so they pass param validation (they never run on
/// this harness's messages).
const SEED: u64 = 0x0315_0303_1500_D00D;

/// The SHIPPED combat tuning parsed from the real `assets/core_tuning/combat.tuning.ron` (so the
/// TU-charge assert reads the REAL `open_door_tu` leaf). Falls back to the default on a parse
/// failure (defensive; the tuning-test suite guards the parse-OK path).
fn shipped_tuning() -> CombatTuning {
    const SHIPPED: &str = include_str!("../../../assets/core_tuning/combat.tuning.ron");
    ron::from_str::<CombatTuning>(SHIPPED).unwrap_or_default()
}

/// Build the focused real-path app wiring three plugins: `MinimalPlugins`, `SimActsPlugin` (owns
/// `dispatch_open_door`), `OpenableTogglePlugin` (owns `apply_openable_toggle`), and
/// `OccupancyMaintenancePlugin` (the GTW-501/502 path/vision projection).
///
/// The `Simulate` band here has NO `BattleInProgress` gate (that lives in `BattleSimPlugin`), so
/// `dispatch_open_door` runs unconditionally over the inserted resources (the gtw525
/// focused-harness pattern). The canonical `with_acts` litany seeds the grids + the sibling
/// dispatchers' battle-lifetime resources (GTW-576) so the projection and the open-door gate
/// read valid state and no sibling trips param validation; the SHIPPED tuning is overlaid so
/// the TU-charge assert reads the REAL `open_door_tu` leaf.
fn open_door_app() -> App {
    let mut app = SimAppBuilder::new()
        .with_seed(SEED)
        .with_acts()
        .with_tuning(shipped_tuning())
        .build();
    app.add_plugins(OpenableTogglePlugin)
        .add_plugins(OccupancyMaintenancePlugin);
    app
}

/// Spawn a standing, alive actor at `at` with a full TU pool of `tu`. Returns its entity. The
/// open-door act reads only the actor's `Position` (adjacency) + `Tu` (afford + charge), so this
/// carries the minimal set.
fn spawn_actor(world: &mut World, at: CellLevel, tu: u8) -> Entity {
    world
        .spawn((
            Position::new(at),
            Faction::new(0),
            Tu::new(tu),
            TuMax::new(tu),
        ))
        .id()
}

/// Spawn a CLOSED openable door at `at` occluding vision at `band` when closed — exactly what
/// `setup_battle` attaches for an Openable piece (`OpenState::Closed` + `OpenableBlocking` + the
/// forced closed blocking pair). Returns its `Entity`.
fn spawn_closed_door(world: &mut World, at: CellLevel, band: HeightBand) -> Entity {
    world
        .spawn((
            TerrainCell::new(at),
            OpenState::Closed,
            OpenableBlocking::new(band),
            BlocksPathfinding,
            BlocksVision::new(band),
        ))
        .id()
}

/// A ground-floor `(x, y)` key.
fn ground(x: i32, y: i32) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(0))
}

/// Read a door's current [`OpenState`], if it still carries one.
fn door_state(app: &App, door: Entity) -> Option<OpenState> {
    app.world().get::<OpenState>(door).copied()
}

/// Read an actor's current `Tu`.
fn tu_of(app: &App, actor: Entity) -> u8 {
    *app.world().get::<Tu>(actor).copied().unwrap_or(Tu::new(0))
}

/// The shipped `open_door_tu` leaf magnitude (read from the tuning resource, never hard-coded).
fn open_door_tu(app: &App) -> u8 {
    app.world()
        .get_resource::<CombatTuning>()
        .map_or(0, |t| *t.open_door_tu)
}

/// Whether the grid reports `at` as PATH-blocked — `None` if the grid resource is absent (kept
/// `Option` so the test never `unwrap`s; the restriction lints fire in tests too).
fn path_blocked(app: &App, at: CellLevel) -> Option<bool> {
    app.world()
        .get_resource::<OccupancyGrid>()
        .map(|g| g.is_path_blocked(&at))
}

/// The grid's vision-occluder band at `at` — `None` if absent OR the cell is not occluding.
fn occluder_band(app: &App, at: CellLevel) -> Option<HeightBand> {
    app.world()
        .get_resource::<OccupancyGrid>()
        .and_then(|g| g.vision_occluder_at(&at))
}

/// Write an [`OpenDoorRequested`] and settle two updates so the buffered message reaches
/// `dispatch_open_door` regardless of intra-frame order (buffered messages persist a frame). A
/// test-body `World` write (`bevy-traps.md` #7 carve-out).
fn open_and_settle(app: &mut App, actor: Entity, door: Entity) {
    app.world_mut()
        .write_message(OpenDoorRequested::new(actor, door));
    app.update();
    app.update();
}

// ── (a) valid request charges exactly OpenDoorTu + toggles CLOSED → open ────────

/// (a): a valid open-door request (an 8-adjacent actor with enough TU, over a CLOSED door) flips
/// the door to `OpenState::Open` and drops the actor's TU by EXACTLY the shipped `open_door_tu`
/// leaf.
#[test]
fn valid_request_toggles_closed_door_open_and_charges_exactly_open_door_tu() {
    let mut app = open_door_app();
    // The actor at (5,5) is orthogonally adjacent to the door at (6,5).
    let actor = spawn_actor(app.world_mut(), ground(5, 5), 100);
    let door = spawn_closed_door(app.world_mut(), ground(6, 5), HeightBand::High);
    app.update();
    let cost = open_door_tu(&app);
    assert!(
        cost > 0,
        "the shipped open_door_tu leaf is a real positive cost"
    );
    let tu_before = tu_of(&app, actor);
    assert_eq!(
        door_state(&app, door),
        Some(OpenState::Closed),
        "the door starts Closed (GTW-470 / GTW-503 default)",
    );

    open_and_settle(&mut app, actor, door);

    assert_eq!(
        door_state(&app, door),
        Some(OpenState::Open),
        "(a): the open-door act toggled the CLOSED door to Open (via the GTW-503 mechanism)",
    );
    assert_eq!(
        tu_of(&app, actor),
        tu_before - cost,
        "(a): the act charged EXACTLY the OpenDoorTu leaf off the actor's pool",
    );
}

/// (a) diagonal: 8-adjacency includes diagonals — a diagonally-adjacent actor opens the door.
#[test]
fn diagonally_adjacent_actor_opens_the_door() {
    let mut app = open_door_app();
    // The actor at (5,5) is DIAGONALLY adjacent to the door at (6,6) (Chebyshev distance 1).
    let actor = spawn_actor(app.world_mut(), ground(5, 5), 100);
    let door = spawn_closed_door(app.world_mut(), ground(6, 6), HeightBand::High);
    app.update();

    open_and_settle(&mut app, actor, door);

    assert_eq!(
        door_state(&app, door),
        Some(OpenState::Open),
        "(a): a diagonally-8-adjacent actor opens the door (Chebyshev-1 includes diagonals)",
    );
}

// ── (b) re-gate rejects: not-adjacent / insufficient TU / already-open ──────────

/// (b) NOT-ADJACENT: an actor two cells away is rejected — the door stays Closed and NO TU is
/// spent.
#[test]
fn non_adjacent_actor_is_rejected_no_toggle_no_charge() {
    let mut app = open_door_app();
    // The actor at (5,5) is TWO cells from the door at (7,5) (Chebyshev distance 2).
    let actor = spawn_actor(app.world_mut(), ground(5, 5), 100);
    let door = spawn_closed_door(app.world_mut(), ground(7, 5), HeightBand::High);
    app.update();
    let tu_before = tu_of(&app, actor);

    open_and_settle(&mut app, actor, door);

    assert_eq!(
        door_state(&app, door),
        Some(OpenState::Closed),
        "(b): a NON-adjacent actor does not open the door (the 8-adjacency gate held)",
    );
    assert_eq!(
        tu_of(&app, actor),
        tu_before,
        "(b): a rejected (non-adjacent) open-door spends NO TU",
    );
}

/// (b) INSUFFICIENT TU: an adjacent actor whose pool is BELOW the `open_door_tu` cost is rejected
/// — the door stays Closed and its (already too-small) TU is untouched.
#[test]
fn insufficient_tu_is_rejected_no_toggle_no_charge() {
    let mut app = open_door_app();
    // The actor is adjacent but has ONE LESS TU than the shipped cost — cannot afford it.
    let broke = open_door_tu(&app).saturating_sub(1);
    let actor = spawn_actor(app.world_mut(), ground(5, 5), broke);
    let door = spawn_closed_door(app.world_mut(), ground(6, 5), HeightBand::High);
    app.update();
    let tu_before = tu_of(&app, actor);

    open_and_settle(&mut app, actor, door);

    assert_eq!(
        door_state(&app, door),
        Some(OpenState::Closed),
        "(b): an actor that cannot afford OpenDoorTu does not open the door (the afford gate held)",
    );
    assert_eq!(
        tu_of(&app, actor),
        tu_before,
        "(b): a rejected (unaffordable) open-door spends NO TU",
    );
}

/// (b) ALREADY-OPEN: an open door is not re-toggled and costs no TU (the button only OPENS —
/// closing is not offered, so an already-open door is a no-op).
#[test]
fn already_open_door_is_a_noop_no_charge() {
    let mut app = open_door_app();
    let actor = spawn_actor(app.world_mut(), ground(5, 5), 100);
    let door = spawn_closed_door(app.world_mut(), ground(6, 5), HeightBand::High);
    app.update();

    // Open it once (a real act).
    open_and_settle(&mut app, actor, door);
    assert_eq!(
        door_state(&app, door),
        Some(OpenState::Open),
        "the door is open after the first act",
    );
    let tu_after_open = tu_of(&app, actor);

    // A second open-door request over the ALREADY-OPEN door: no re-toggle, no further charge.
    open_and_settle(&mut app, actor, door);
    assert_eq!(
        door_state(&app, door),
        Some(OpenState::Open),
        "(b): an already-open door stays Open (the closed-gate held — the button only opens)",
    );
    assert_eq!(
        tu_of(&app, actor),
        tu_after_open,
        "(b): opening an already-open door spends NO further TU",
    );
}

// ── (c) after opening, the door no longer blocks pathing/vision ─────────────────

/// (c): after the open-door act, the door no longer blocks PATH or occludes VISION — asserted
/// through the EXISTING GTW-503 downstream (`apply_openable_toggle` → the GTW-501/502
/// change-detection). A CLOSED door blocks path + occludes vision; the act clears BOTH.
#[test]
fn opening_the_door_clears_path_and_vision_via_gtw503_downstream() {
    let at = ground(6, 5);
    let mut app = open_door_app();
    let actor = spawn_actor(app.world_mut(), ground(5, 5), 100);
    let door = spawn_closed_door(app.world_mut(), at, HeightBand::High);
    // Settle the spawn-inserted blocking pair (Added fires on the first tick's projection).
    app.update();
    assert_eq!(
        path_blocked(&app, at),
        Some(true),
        "(c): a CLOSED door blocks the path before opening",
    );
    assert_eq!(
        occluder_band(&app, at),
        Some(HeightBand::High),
        "(c): a CLOSED door occludes vision before opening",
    );

    // Open the door via the act — the settle covers the act tick, the toggle's deferred Commands,
    // and the GTW-501/502 re-projection (buffered messages + Commands persist across the ticks).
    open_and_settle(&mut app, actor, door);
    // One more tick so the toggle's deferred component removal is projected by GTW-501/502.
    app.update();

    assert_eq!(
        door_state(&app, door),
        Some(OpenState::Open),
        "(c): the act opened the door",
    );
    assert_eq!(
        path_blocked(&app, at),
        Some(false),
        "(c): an OPEN door no longer blocks the path (the GTW-503 downstream cleared it)",
    );
    assert_eq!(
        occluder_band(&app, at),
        None,
        "(c): an OPEN door no longer occludes vision (the GTW-503 downstream cleared it)",
    );
}

/// (b) panic-free: an [`OpenDoorRequested`] for a NON-openable entity (no `OpenState`) is skipped
/// without panic and charges nothing.
#[test]
fn non_openable_target_is_skipped_panic_free() {
    let mut app = open_door_app();
    let actor = spawn_actor(app.world_mut(), ground(5, 5), 100);
    // A plain terrain cell adjacent to the actor, but NOT openable (no OpenState).
    let plain: Entity = app.world_mut().spawn(TerrainCell::new(ground(6, 5))).id();
    app.update();
    let tu_before = tu_of(&app, actor);

    open_and_settle(&mut app, actor, plain);

    assert!(
        door_state(&app, plain).is_none(),
        "(b): a non-openable target gains no OpenState from a stray OpenDoorRequested",
    );
    assert_eq!(
        tu_of(&app, actor),
        tu_before,
        "(b): an open-door over a non-openable target spends NO TU (panic-free skip)",
    );
}
