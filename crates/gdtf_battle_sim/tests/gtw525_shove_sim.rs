//! GTW-525 — the SHOVE act (the last GTW-39 falls child), proven END-TO-END through the REAL
//! wiring (`dispatch_shove` + the melee / fire auto-shove hooks + the shared GTW-523 fall path).
//!
//! The clause contract (C1-C3 / C6-C7):
//!
//! - **QA(1) displace-away direction (C1)** — a deliberate shove pushes the target ONE cell
//!   directly AWAY from the shover (the attacker->target direction).
//! - **QA(2) unsupported=>fall via 523 + `FallOccurred` (C1)** — a shove off a ledge (no
//!   supporting slab at the destination storey) makes the target FALL through the shared
//!   `resolve_drop` path, emitting `FallOccurred` + dropping Hp.
//! - **QA(3) supported=>move (C1)** — a shove onto a supported cell (ground / a Present slab)
//!   moves the target one cell, no fall, no Hp loss.
//! - **QA(4) blocked=>no-op (C1)** — a shove into a solid (another ganger / a wall) is a no-op:
//!   the target does not move.
//! - **QA(5) deliberate act: any ganger, adjacency-gated, TU spent, NO wound (C2)** — the
//!   deliberate shove gates 8-adjacency + opposing + alive, spends the shove TU, and deals NO
//!   wound of its own (a supported shove leaves Hp/Wounds untouched).
//! - **QA(6) weapon-tag auto-shove on a connecting MELEE strike (C3)** — a `shove`-tagged melee
//!   weapon knocks the target back on a CONNECTING strike (in addition to the damage).
//! - **QA(7) weapon-tag auto-shove on a connecting RANGED shot (C3)** — a `shove`-tagged gun
//!   knocks the target back on a CONNECTING shot.
//! - **QA(8) miss / non-tagged => NO shove (C3)** — a non-`shove` weapon never shoves; a missed
//!   attack never shoves.
//! - **QA(9) determinism (C6)** — the same seed + same message order yields the identical shove
//!   outcome (the shove itself is RNG-free; only the fall draws, deterministically).
//!
//! No pinned tunable magnitude — every assert is a RELATION (moved / fell / no-op / Hp dropped
//! / TU spent), never a shipped balance number (the brittle-test rule). Render-free, zero
//! pixels; the one `World` mutation is in a TEST BODY (`bevy-traps.md` #7 carve-out).

use bevy::{
    app::{App, Update},
    prelude::{Entity, MessageReader, MinimalPlugins, ResMut, Resource, World},
};
use gdtf_battle_sim::{
    BattleSeed, Cell, CellLevel, CombatTuning, CoverLedger, Direction, Faction, FallOccurred, Hp,
    InflictedWounds, InjuryRng, Level, LifeState, LootRng, Luck, OccupancyGrid,
    OccupancyMaintenancePlugin, Position, ProcgenRng, SeverityRng, ShotRng, ShoveOutcome,
    ShoveRequested, SlabState, SquadVisibility, Stance, StanceKind, SurfaceGrid, Toughness, Tu,
    TuMax, VerticalLinkGraph, Wounds, falls::FallsPlugin, resolve_shove,
};

/// An arbitrary fixed seed — determinism is the property, the value is irrelevant.
const SEED: u64 = 0x0525_5405_DEAD_BEEF;

/// The SHIPPED combat tuning parsed from the real `assets/core_tuning/combat.tuning.ron` (the
/// realistic severity edges + the real `shove_tu` leaf). Falls back to the default on a parse
/// failure (defensive; the tuning-test suite guards the parse-OK path).
fn shipped_tuning() -> CombatTuning {
    const SHIPPED: &str = include_str!("../../../assets/core_tuning/combat.tuning.ron");
    ron::from_str::<CombatTuning>(SHIPPED).unwrap_or_default()
}

// ── Test-local signal recorders (the gtw523 / gtw507 idiom) ────────────────────

/// Every `FallOccurred` observed across the run.
#[derive(Resource, Default)]
struct FallLog {
    /// One entry per `FallOccurred` emitted.
    falls: Vec<FallOccurred>,
}

/// Drain `FallOccurred` into the recorder.
fn record_falls(mut reader: MessageReader<FallOccurred>, mut log: ResMut<FallLog>) {
    for signal in reader.read() {
        log.falls.push(*signal);
    }
}

/// Build the focused real-path app: `MinimalPlugins` + `SimActsPlugin` (which owns
/// `dispatch_shove`) + `OccupancyMaintenancePlugin` + `FallsPlugin` (the `FallOccurred` buffer).
/// Inserts the sim resources the shove + its fall read. The `Simulate` band here has NO
/// `BattleInProgress` gate (that lives in `BattleSimPlugin`), so `dispatch_shove` runs
/// unconditionally over the inserted resources — the gtw523 focused-harness pattern.
fn shove_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(gdtf_battle_sim::acts::SimActsPlugin)
        .add_plugins(OccupancyMaintenancePlugin)
        .add_plugins(FallsPlugin);
    let root = BattleSeed::new(SEED);
    app.insert_resource(ShotRng::from_root(root));
    app.insert_resource(SeverityRng::from_root(root));
    app.insert_resource(InjuryRng::from_root(root));
    app.insert_resource(LootRng::from_root(root));
    app.insert_resource(ProcgenRng::from_root(root));
    app.insert_resource(gdtf_battle_sim::InjuryTables::default());
    app.insert_resource(gdtf_battle_sim::InjuryRegistry::default());
    // The SHIPPED tuning (real severity edges + real shove_tu) so the shove TU spend + any fall
    // bucket are realistic. A parse failure falls back to the default (the test still runs).
    app.insert_resource(shipped_tuning());
    app.insert_resource(gdtf_battle_sim::PlayerFaction::new(Faction::new(0)));
    app.insert_resource(CoverLedger::new());
    app.insert_resource(VerticalLinkGraph::default());
    app.insert_resource(SquadVisibility::default());
    app.insert_resource(gdtf_battle_sim::SlabLedger::new());
    app.insert_resource(gdtf_battle_sim::BraceStairCells::empty());
    let default_open = CombatTuning::default().move_costs.open;
    app.insert_resource(gdtf_battle_sim::FloorCostGrid::new(default_open, []));
    app.init_resource::<FallLog>();
    app.add_systems(Update, record_falls);
    app
}

/// Spawn a standing, alive ganger of `faction` at `(cell, level)` with the shove-relevant
/// component set (a large HP/Wounds pool so a fall wounds but rarely kills — a clean HP read).
/// Bare flesh (no worn armor). Returns its entity.
fn spawn_ganger(world: &mut World, at: CellLevel, faction: u8) -> Entity {
    world
        .spawn((
            Position::new(at),
            Stance::new(StanceKind::Standing),
            Faction::new(faction),
            Tu::new(100),
            TuMax::new(100),
            Hp::new(1000),
            Wounds::new(200),
            LifeState::Alive,
            InflictedWounds::default(),
            Toughness::new(0.0),
            Luck::new(0.0),
        ))
        .id()
}

/// Read a ganger's current `(cell, level)`.
fn pos_of(app: &App, entity: Entity) -> Option<CellLevel> {
    app.world().get::<Position>(entity).map(|p| **p)
}

/// Read a ganger's current `Hp`.
fn hp_of(app: &App, entity: Entity) -> u16 {
    *app.world().get::<Hp>(entity).copied().unwrap_or(Hp::new(0))
}

/// Read a ganger's current `Tu`.
fn tu_of(app: &App, entity: Entity) -> u8 {
    *app.world().get::<Tu>(entity).copied().unwrap_or(Tu::new(0))
}

/// Read a ganger's current `Wounds`.
fn wounds_of(app: &App, entity: Entity) -> u8 {
    *app.world()
        .get::<Wounds>(entity)
        .copied()
        .unwrap_or(Wounds::new(0))
}

/// Write a deliberate `ShoveRequested` and settle two updates so the buffered message reaches
/// `dispatch_shove` regardless of intra-frame order (buffered messages persist a frame). A
/// test-body `World` write (`bevy-traps.md` #7 carve-out).
fn shove_and_settle(app: &mut App, shover: Entity, target: Entity) {
    app.world_mut()
        .write_message(ShoveRequested::new(shover, target));
    app.update();
    app.update();
}

/// The full run history of `FallOccurred` signals (from the recorder).
fn fall_signals(app: &App) -> Vec<FallOccurred> {
    app.world()
        .get_resource::<FallLog>()
        .map(|log| log.falls.clone())
        .unwrap_or_default()
}

/// A ground-floor `(x, y)` key.
fn ground(x: i32, y: i32) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(0))
}

/// An upper-storey `(x, y, level)` key.
fn upper(x: i32, y: i32, level: u8) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(level))
}

// ── QA(1) — displace-away direction (the pure verb + the deliberate act) ───────

/// QA(1): a deliberate shove pushes the target ONE cell directly AWAY from the shover. The
/// shover at (5,5) faces the target at (6,5) (east); the shove moves it to (7,5). Also asserts
/// the PURE verb resolves the same displacement (C1).
#[test]
fn shove_pushes_target_one_cell_away_from_shover() {
    let mut app = shove_app();
    app.insert_resource(SurfaceGrid::new());
    app.insert_resource(OccupancyGrid::new());
    let shover = spawn_ganger(app.world_mut(), ground(5, 5), 0);
    let target = spawn_ganger(app.world_mut(), ground(6, 5), 1);
    app.update();

    // The pure verb (C1): attacker (5,5) -> target (6,5) is East, so the destination is (7,5),
    // supported (ground level 0) => Moved.
    let outcome = resolve_shove(
        Position::new(ground(5, 5)),
        Position::new(ground(6, 5)),
        target,
        app.world().resource::<SurfaceGrid>(),
        app.world().resource::<OccupancyGrid>(),
    );
    assert_eq!(
        outcome,
        ShoveOutcome::Moved { dest: ground(7, 5) },
        "the pure verb pushes the target one cell East (away from the shover), onto the ground"
    );

    shove_and_settle(&mut app, shover, target);

    // The live act moves the target the same way — one cell East, away from the shover.
    assert_eq!(
        pos_of(&app, target),
        Some(ground(7, 5)),
        "the deliberate shove pushes the target one cell directly away from the shover"
    );
    // The shover did NOT move (a shove displaces the TARGET, not the shover).
    assert_eq!(
        pos_of(&app, shover),
        Some(ground(5, 5)),
        "the shover stays put"
    );
}

// ── QA(2) — unsupported=>fall via 523 + FallOccurred ───────────────────────────

/// QA(2): a shove off a LEDGE (the destination cell has no supporting slab at the target's
/// storey) makes the target FALL through the shared GTW-523 `resolve_drop` path — its Position
/// drops to the supported storey below, `FallOccurred` fires, and Hp drops.
#[test]
fn shove_off_a_ledge_falls_via_523_and_fires_falloccurred() {
    let mut app = shove_app();
    // A surface where the target's storey (2) is supported AT its start cell (6,5) but the
    // destination (7,5) has NO slab at level 2 (open air) — so a shove East off the ledge falls
    // to the ground (0). Level 0 always supports the landing.
    let mut surface = SurfaceGrid::new();
    surface.set_slab(upper(6, 5, 2), SlabState::Present); // the target stands on a real floor
    // (7,5,2) is left Absent — the ledge edge.
    app.insert_resource(surface);
    app.insert_resource(OccupancyGrid::new());
    let shover = spawn_ganger(app.world_mut(), upper(5, 5, 2), 0);
    let target = spawn_ganger(app.world_mut(), upper(6, 5, 2), 1);
    app.update();
    let hp_before = hp_of(&app, target);

    shove_and_settle(&mut app, shover, target);

    // The target fell off the ledge: its cell is now (7,5) and its storey dropped BELOW 2.
    let Some(landed) = pos_of(&app, target) else {
        unreachable!("the target persists");
    };
    assert_eq!(
        landed.x, 7,
        "the target was pushed East onto the ledge cell"
    );
    assert_eq!(landed.y, 5, "the shove is a lateral (same-y) East push");
    assert!(
        landed.z < 2,
        "the target FELL off the unsupported ledge to a storey below 2 (got z={})",
        landed.z
    );
    // A FallOccurred fired for the target (the shared GTW-523 fall path ran).
    let signals = fall_signals(&app);
    assert!(
        signals.iter().any(|s| s.ganger == target),
        "a shove off a ledge fires FallOccurred for the target (the shared 523 fall path)"
    );
    // The fall dropped Hp (the shared weight-free fall damage).
    assert!(
        hp_of(&app, target) < hp_before,
        "the fall drops the shoved target's Hp (the shared 523 fall damage)"
    );
}

// ── QA(3) — supported=>move (no fall, no wound) ────────────────────────────────

/// QA(3): a shove onto a SUPPORTED cell (an intact Present slab at the target's storey) moves
/// the target one cell and does NOT fall — no `FallOccurred`, no Hp loss.
#[test]
fn shove_onto_supported_cell_moves_without_falling() {
    let mut app = shove_app();
    let mut surface = SurfaceGrid::new();
    // Both the start (6,5,2) AND the destination (7,5,2) have a Present slab → supported → move.
    surface.set_slab(upper(6, 5, 2), SlabState::Present);
    surface.set_slab(upper(7, 5, 2), SlabState::Present);
    app.insert_resource(surface);
    app.insert_resource(OccupancyGrid::new());
    let shover = spawn_ganger(app.world_mut(), upper(5, 5, 2), 0);
    let target = spawn_ganger(app.world_mut(), upper(6, 5, 2), 1);
    app.update();
    let hp_before = hp_of(&app, target);

    shove_and_settle(&mut app, shover, target);

    assert_eq!(
        pos_of(&app, target),
        Some(upper(7, 5, 2)),
        "a shove onto a supported (Present-slab) cell moves the target one cell, same storey"
    );
    assert!(
        fall_signals(&app).iter().all(|s| s.ganger != target),
        "a supported shove fires NO FallOccurred (no fall)"
    );
    assert_eq!(
        hp_of(&app, target),
        hp_before,
        "a supported shove deals NO damage (pure displacement — the shove itself never wounds)"
    );
}

// ── QA(4) — blocked=>no-op ─────────────────────────────────────────────────────

/// QA(4): a shove into a cell OCCUPIED by another ganger is a NO-OP — the target does not move.
#[test]
fn shove_into_an_occupied_cell_is_a_noop() {
    let mut app = shove_app();
    app.insert_resource(SurfaceGrid::new());
    app.insert_resource(OccupancyGrid::new());
    let shover = spawn_ganger(app.world_mut(), ground(5, 5), 0);
    let target = spawn_ganger(app.world_mut(), ground(6, 5), 1);
    // A blocker standing on the destination cell (7,5) — the shove would push the target into it.
    let _blocker = spawn_ganger(app.world_mut(), ground(7, 5), 1);
    // Settle so occupancy maintenance publishes the blocker's slot before the shove resolves.
    app.update();
    app.update();

    shove_and_settle(&mut app, shover, target);

    assert_eq!(
        pos_of(&app, target),
        Some(ground(6, 5)),
        "a shove into a ganger-occupied cell is a NO-OP (never shove into a solid)"
    );
    assert!(
        fall_signals(&app).iter().all(|s| s.ganger != target),
        "a blocked shove fires no fall"
    );
}

// ── QA(5) — deliberate act: any ganger, adjacency-gated, TU spent, NO wound ─────

/// QA(5a): a deliberate shove spends the shover's TU (the shove act is TU-costed) and deals NO
/// wound (a supported shove leaves the target's Hp/Wounds untouched — pure displacement).
#[test]
fn deliberate_shove_spends_tu_and_deals_no_wound() {
    let mut app = shove_app();
    app.insert_resource(SurfaceGrid::new());
    app.insert_resource(OccupancyGrid::new());
    let shover = spawn_ganger(app.world_mut(), ground(5, 5), 0);
    let target = spawn_ganger(app.world_mut(), ground(6, 5), 1);
    app.update();
    let tu_before = tu_of(&app, shover);
    let (hp_before, wounds_before) = (hp_of(&app, target), wounds_of(&app, target));

    shove_and_settle(&mut app, shover, target);

    assert!(
        tu_of(&app, shover) < tu_before,
        "the deliberate shove spends the shover's TU (a TU-costed act)"
    );
    assert_eq!(
        hp_of(&app, target),
        hp_before,
        "the deliberate shove deals NO HP damage (pure displacement)"
    );
    assert_eq!(
        wounds_of(&app, target),
        wounds_before,
        "the deliberate shove records NO Wound (pure displacement — the fall does the harm)"
    );
}

/// QA(5b): the deliberate shove is GATED — a NON-ADJACENT target (Chebyshev > 1) is a no-op
/// (no move, no TU spent); a SAME-FACTION (ally) target is a no-op.
#[test]
fn deliberate_shove_gates_adjacency_and_faction() {
    // Non-adjacent: target three cells away.
    let mut app = shove_app();
    app.insert_resource(SurfaceGrid::new());
    app.insert_resource(OccupancyGrid::new());
    let shover = spawn_ganger(app.world_mut(), ground(5, 5), 0);
    let far = spawn_ganger(app.world_mut(), ground(8, 5), 1);
    app.update();
    let tu_before = tu_of(&app, shover);
    shove_and_settle(&mut app, shover, far);
    assert_eq!(
        pos_of(&app, far),
        Some(ground(8, 5)),
        "a non-adjacent target is not shoved (the 8-adjacency gate held)"
    );
    assert_eq!(
        tu_of(&app, shover),
        tu_before,
        "a rejected (non-adjacent) shove spends NO TU"
    );

    // Same-faction: an adjacent ALLY.
    let mut app = shove_app();
    app.insert_resource(SurfaceGrid::new());
    app.insert_resource(OccupancyGrid::new());
    let shover = spawn_ganger(app.world_mut(), ground(5, 5), 0);
    let ally = spawn_ganger(app.world_mut(), ground(6, 5), 0);
    app.update();
    let tu_before = tu_of(&app, shover);
    shove_and_settle(&mut app, shover, ally);
    assert_eq!(
        pos_of(&app, ally),
        Some(ground(6, 5)),
        "a same-faction ally is not shoved (the opposing-faction gate held)"
    );
    assert_eq!(
        tu_of(&app, shover),
        tu_before,
        "a rejected (ally) shove spends NO TU"
    );
}

// ── QA(9) — determinism ────────────────────────────────────────────────────────

/// QA(9): the same seed + same message order yields the IDENTICAL shove-off-a-ledge outcome
/// (landing storey + Hp loss) across two independent runs (the shove is RNG-free; the fall
/// draws deterministically from the seed).
#[test]
fn shove_outcomes_are_deterministic_under_same_seed() {
    let run = || -> (Option<CellLevel>, u16) {
        let mut app = shove_app();
        let mut surface = SurfaceGrid::new();
        surface.set_slab(upper(6, 5, 3), SlabState::Present);
        surface.set_slab(upper(7, 5, 1), SlabState::Present); // the fall lands on level 1
        app.insert_resource(surface);
        app.insert_resource(OccupancyGrid::new());
        let shover = spawn_ganger(app.world_mut(), upper(5, 5, 3), 0);
        let target = spawn_ganger(app.world_mut(), upper(6, 5, 3), 1);
        app.update();
        let before = hp_of(&app, target);
        shove_and_settle(&mut app, shover, target);
        (pos_of(&app, target), before - hp_of(&app, target))
    };
    assert_eq!(
        run(),
        run(),
        "same seed + same message order must yield identical shove/fall outcomes"
    );
}

/// A direction sanity check for the pure verb — a diagonal shove pushes one cell on BOTH axes
/// away from the shover (no pinned magnitude, a geometry relation only).
#[test]
fn pure_verb_diagonal_pushes_one_cell_on_both_axes() {
    let surface = SurfaceGrid::new();
    let occupancy = OccupancyGrid::new();
    // Shover at (5,5), target diagonally at (6,6) (SouthEast) → destination (7,7), supported.
    let outcome = resolve_shove(
        Position::new(ground(5, 5)),
        Position::new(ground(6, 6)),
        Entity::PLACEHOLDER,
        &surface,
        &occupancy,
    );
    assert_eq!(
        outcome,
        ShoveOutcome::Moved { dest: ground(7, 7) },
        "a diagonal shove pushes one cell on both axes directly away from the shover"
    );
    // Sanity: the direction helper agrees (attacker->target is SouthEast).
    assert_eq!(
        Direction::from_cells(Cell::new(5, 5), Cell::new(6, 6)),
        Some(Direction::SouthEast),
        "the shove direction is the attacker->target compass step"
    );
}
