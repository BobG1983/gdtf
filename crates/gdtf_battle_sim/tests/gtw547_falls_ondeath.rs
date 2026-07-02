//! GTW-547 (on-death effects, child GTW-41g) — the FALLS terminal-death gate on the LIVE
//! path: a ganger KILLED by a fall (its `Wounds` reach 0 through the shared wound-synthesis
//! core) emits an [`OnDeathOccurred`] at its landing cell, exactly like the ranged / melee /
//! bleed / DOT / field kills, so `resolve_on_death` can fan its authored on-death effect.
//!
//! A fall CAN kill (the shared core flips [`LifeState::Dead`] on `Wounds -> 0`), so falls is a
//! terminal death gate the ticket's "`OnDeathOccurred` from EVERY terminal gate" scope
//! clarification covers. Before this slice `apply_falls` emitted only `FallOccurred` +
//! `InjuryInflicted` — a fall-killed ganger silently skipped its on-death effect. This test
//! PINS the emission: reverting the `apply_falls` `deaths.write(...)` leaves the faller Dead
//! but the captured-death assertion fails.
//!
//! HARNESS NOTE: this drives the REAL [`apply_falls`] system on the same bespoke falls harness
//! the GTW-523 integration test uses (a `MinimalPlugins` + `SimActsPlugin` + falls/occupancy
//! plugins app) — `apply_falls`'s resource set (a `SurfaceGrid` column, the seeded RNG streams,
//! the injury content) is falls-specific, so the `setup_battle_on_request` harness the sibling
//! Explode / `LeaveField` tests use does not fit it. `SimActsPlugin` registers the
//! `OnDeathOccurred` buffer (the same slot the app wires it in).

use bevy::{
    app::{App, Update},
    prelude::{Entity, MessageReader, MinimalPlugins, ResMut, Resource},
};
use gdtf_battle_sim::{
    BattleSeed, Cell, CellLevel, CombatTuning, CoverLedger, Faction, Hp, InflictedWounds,
    InjuryRng, Level, LifeState, LootRng, Luck, OccupancyGrid, OccupancyMaintenancePlugin,
    PerStoreyDamage, PlayerFaction, Position, ProcgenRng, SeverityRng, ShotRng, SquadVisibility,
    Stance, StanceKind, SurfaceGrid, Toughness, Tu, TuMax, VerticalLinkGraph, Wounds,
    falls::FallsPlugin, occupancy_sync::SlabDestroyed, on_death::OnDeathOccurred,
};

/// An arbitrary fixed seed — determinism is the property, the value is irrelevant.
const SEED: u64 = 0x0547_FA11_DEAD_BEEF;
/// The shared column x/y the faller stands in (the cell the destroyed slab keys).
const COL_X: i32 = 5;
/// The shared column x/y the faller stands in.
const COL_Y: i32 = 5;

/// The `(cell, level)` key of the destroyed slab / the standing faller's storey.
const fn column_cell() -> Cell {
    Cell::new(COL_X, COL_Y)
}

/// Every [`OnDeathOccurred`] observed across the run — a test-local recorder so the assertion
/// reads the full run history rather than racing the one-update message lifetime (the gtw523
/// `FallLog` precedent).
#[derive(Resource, Default)]
struct DeathLog {
    /// One entry per `OnDeathOccurred` emitted.
    deaths: Vec<OnDeathOccurred>,
}

/// Drain `OnDeathOccurred` into the recorder — added after the plugins so the buffer exists.
fn record_deaths(mut reader: MessageReader<OnDeathOccurred>, mut log: ResMut<DeathLog>) {
    for death in reader.read() {
        log.deaths.push(*death);
    }
}

/// Build the bespoke falls harness with `seed` and a lethal `per_storey` magnitude. Uses the
/// LOW `CombatTuning::default()` severity ladder (which turns even a shallow fall Fatal — the
/// gtw523 harness note) + a large per-storey magnitude so a multi-storey fall reliably KILLS.
fn falls_app(seed: u64, per_storey: PerStoreyDamage) -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(gdtf_battle_sim::acts::SimActsPlugin)
        .add_plugins(OccupancyMaintenancePlugin)
        .add_plugins(FallsPlugin);
    let root = BattleSeed::new(seed);
    // The two streams the fall fold draws (SeverityRng + InjuryRng) + the others the sibling
    // Simulate-band systems read (ShotRng / LootRng / ProcgenRng).
    app.insert_resource(ShotRng::from_root(root));
    app.insert_resource(SeverityRng::from_root(root));
    app.insert_resource(InjuryRng::from_root(root));
    app.insert_resource(LootRng::from_root(root));
    app.insert_resource(ProcgenRng::from_root(root));
    app.insert_resource(gdtf_battle_sim::InjuryTables::default());
    app.insert_resource(gdtf_battle_sim::InjuryRegistry::default());
    // The LOW default ladder makes even a shallow fall Fatal (the gtw523 harness note), with a
    // large per-storey magnitude overlaid so the multi-storey fall is unambiguously lethal.
    app.insert_resource(CombatTuning {
        per_storey_damage: per_storey,
        ..Default::default()
    });
    app.insert_resource(PlayerFaction::new(Faction::new(1)));
    app.insert_resource(CoverLedger::new());
    app.insert_resource(VerticalLinkGraph::default());
    app.insert_resource(SquadVisibility::default());
    // The sibling Simulate-band dispatch systems (dispatch_fire / dispatch_move) read these —
    // the band runs unconditionally here (no BattleInProgress gate outside BattleSimPlugin),
    // so every member's Res must exist even though this test only exercises apply_falls.
    app.insert_resource(gdtf_battle_sim::SlabLedger::new());
    app.insert_resource(gdtf_battle_sim::BraceStairCells::empty());
    let default_open = CombatTuning::default().move_costs.open;
    app.insert_resource(gdtf_battle_sim::FloorCostGrid::new(default_open, []));
    app.init_resource::<DeathLog>();
    app.add_systems(Update, record_deaths);
    app
}

/// Spawn a standing, alive faller at `(column, level)` with a SHALLOW Wounds pool (1 Wound,
/// low Hp) so a multi-storey fall's synthesized blow reliably drains it to 0 → Dead. Bare flesh
/// (no worn armor). Returns its entity.
fn spawn_frail_faller(app: &mut App, level: u8) -> Entity {
    app.world_mut()
        .spawn((
            Position::new(CellLevel::new(column_cell(), Level::new(level))),
            Stance::new(StanceKind::Standing),
            Faction::new(1),
            Tu::new(100),
            TuMax::new(100),
            Hp::new(4),
            Wounds::new(1),
            LifeState::Alive,
            InflictedWounds::default(),
            Toughness::new(0.0),
            Luck::new(0.0),
        ))
        .id()
}

/// Write a `SlabDestroyed` at the column cell + `level`, then drive two updates so the buffered
/// signal reaches `apply_falls` regardless of intra-frame order (a test-body `World` write —
/// `bevy-traps.md` #7 carve-out).
fn destroy_slab_and_settle(app: &mut App, level: u8) {
    app.world_mut()
        .write_message(SlabDestroyed::new(CellLevel::new(
            column_cell(),
            Level::new(level),
        )));
    app.update();
    app.update();
}

/// The current `LifeState` of `entity` (Alive default if absent — no `unwrap`).
fn life_of(app: &App, entity: Entity) -> LifeState {
    app.world()
        .get::<LifeState>(entity)
        .copied()
        .unwrap_or(LifeState::Alive)
}

/// Whether the recorded deaths include one for `entity` — the gate-emission assertion.
fn death_emitted_for(app: &App, entity: Entity) -> bool {
    app.world()
        .get_resource::<DeathLog>()
        .is_some_and(|log| log.deaths.iter().any(|d| d.entity == entity))
}

/// The falls terminal-death gate: a ganger KILLED by a fall emits `OnDeathOccurred` at its
/// landing cell, so `resolve_on_death` fans its authored on-death effect (the ticket's
/// every-terminal-gate scope clarification). PIN-DISCRIMINATING: the faller flips to Dead
/// regardless of the emit, but reverting the `apply_falls` `deaths.write(...)` fails the
/// captured-death assertion.
#[test]
fn a_ganger_killed_by_a_fall_emits_on_death() {
    // A high per-storey magnitude over the low default ladder → a 2-storey fall is lethal.
    let mut app = falls_app(SEED, PerStoreyDamage::new(1000));
    // A frail faller STANDING on the floor of storey 2 (Position.level == 2).
    let faller = spawn_frail_faller(&mut app, 2);
    // An empty surface grid (no intermediate slabs) → the faller drops to the ground (0): a
    // 2-storey fall through open air.
    app.insert_resource(SurfaceGrid::new());
    app.insert_resource(OccupancyGrid::new());

    destroy_slab_and_settle(&mut app, 2);

    // The fall KILLED the faller (its Wounds bled to 0 through the shared wound core).
    assert_eq!(
        life_of(&app, faller),
        LifeState::Dead,
        "a lethal multi-storey fall kills the frail faller"
    );
    // The falls gate emitted OnDeathOccurred for the fall-killed faller (the gate under test).
    assert!(
        death_emitted_for(&app, faller),
        "apply_falls emits OnDeathOccurred for a fall-killed faller (the falls terminal-death \
         gate) so resolve_on_death can fan its on-death effect"
    );
}
