//! Shared GTW-523 falls fixture: the falls app, the faller spawner, the
//! slab-destruction driver, the signal recorders, and the fall-signal reader.

use bevy::{
    app::{App, Update},
    prelude::{Entity, MessageReader, ResMut, Resource, World},
};
use gdtf_battle_sim::{
    Cell, CellLevel, CombatTuning, Faction, FallOccurred, Hp, InflictedWounds, Level, LifeState,
    Luck, OccupancyMaintenancePlugin, PerStoreyDamage, Position, Stance, StanceKind, Toughness, Tu,
    TuMax, Wounds, acts::InjuryInflicted, falls::FallsPlugin, occupancy_sync::SlabDestroyed,
    test_support::SimAppBuilder,
};

/// The SHIPPED combat tuning parsed from the real `assets/core_tuning/combat.tuning.ron` (the
/// realistic severity edges / scaling — NOT the low `CombatTuning::default()` placeholder
/// ladder). Falls back to the default on a parse failure (defensive; the tuning-test suite
/// already guards the parse-OK path).
pub(crate) fn shipped_tuning() -> CombatTuning {
    const SHIPPED: &str = include_str!("../../../../assets/core_tuning/combat.tuning.ron");
    ron::from_str::<CombatTuning>(SHIPPED).unwrap_or_default()
}

/// The shared column x/y every faller stands in (the cell the destroyed slab keys).
pub(crate) const COL_X: i32 = 5;
pub(crate) const COL_Y: i32 = 5;

/// The `(cell, level)` key of the destroyed slab / the standing faller's storey.
pub(crate) const fn column_cell() -> Cell {
    Cell::new(COL_X, COL_Y)
}

/// An arbitrary fixed seed — determinism is the property, the value is irrelevant.
pub(crate) const SEED: u64 = 0x0523_FA11_DEAD_BEEF;

// ── Test-local signal recorders ───────────────────────────────────────────────
//
// A `MessageReader` sees only the current+previous update, so recording every signal into a
// resource the moment it is emitted lets the asserts read the full run history (the `melee_act`
// `MeleeLog` precedent).

/// Every `FallOccurred` observed across the run.
#[derive(Resource, Default)]
pub(crate) struct FallLog {
    /// One entry per `FallOccurred` emitted.
    falls: Vec<FallOccurred>,
}

/// Every `InjuryInflicted` target observed across the run.
#[derive(Resource, Default)]
pub(crate) struct InjuryLog {
    /// The entity of each `InjuryInflicted` emitted.
    pub(crate) targets: Vec<Entity>,
}

/// Drain `FallOccurred` into the recorder.
pub(crate) fn record_falls(mut reader: MessageReader<FallOccurred>, mut log: ResMut<FallLog>) {
    for signal in reader.read() {
        log.falls.push(*signal);
    }
}

/// Drain `InjuryInflicted` into the recorder (targets only — the applier drains the buffer
/// too, but each `MessageReader` has its own cursor, so both see every message).
pub(crate) fn record_injuries(
    mut reader: MessageReader<InjuryInflicted>,
    mut log: ResMut<InjuryLog>,
) {
    for message in reader.read() {
        log.targets.push(message.target);
    }
}

/// Build the real-path app: `MinimalPlugins` + `SimActsPlugin` (the `dispatch_fire`
/// `apply_falls` orders `.after`) + `OccupancyMaintenancePlugin` (the `sync_destroyed_slab`
/// it orders `.after`) + the GTW-523 `FallsPlugin` (the `FallOccurred` buffer +
/// `apply_falls`). Inserts the sim resources the falls fold + the sibling dispatch systems
/// read. The `Simulate` band here has NO `BattleInProgress` gate (that gate lives in
/// `BattleSimPlugin`), so `apply_falls` runs unconditionally over the inserted resources —
/// the focused-harness pattern the fire/slab bridge tests use.
pub(crate) fn falls_app(seed: u64, per_storey: PerStoreyDamage) -> App {
    // The canonical `with_acts` litany (GTW-576) seeds the grids + five RNG streams + empty
    // injury content the whole Simulate band validates against. Overlaid here: the SHIPPED
    // combat tuning (the real severity edges / scaling — the `CombatTuning::default()` is a
    // low placeholder ladder that turns even a shallow fall Fatal) with the per-storey
    // magnitude under test (QA(7) varies it), and a non-player PlayerFaction.
    let mut tuning = shipped_tuning();
    tuning.per_storey_damage = per_storey;
    let mut app = SimAppBuilder::new()
        .with_seed(seed)
        .with_acts()
        .with_player_faction(1)
        .with_tuning(tuning)
        .build();
    app.add_plugins(OccupancyMaintenancePlugin)
        .add_plugins(FallsPlugin);
    // The signal recorders — added after the plugins so the FallOccurred / InjuryInflicted
    // buffers exist, so the run's full history is queryable after the app.update()s.
    app.init_resource::<FallLog>();
    app.init_resource::<InjuryLog>();
    app.add_systems(Update, (record_falls, record_injuries));
    app
}

/// Spawn a standing, alive faller with the falls-query + fold component set at `(cell,
/// level)`, with a large HP/Wounds pool (so a fall wounds but rarely kills — a clean HP
/// measurement). Bare flesh (no worn armor). Returns its entity.
pub(crate) fn spawn_faller(world: &mut World, level: u8) -> Entity {
    world
        .spawn((
            Position::new(CellLevel::new(column_cell(), Level::new(level))),
            Stance::new(StanceKind::Standing),
            Faction::new(1),
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

/// Read a faller's current `Position` level (its storey index) — via the canonical
/// `CellLevel::level` accessor (GTW-565).
pub(crate) fn level_of(app: &App, entity: Entity) -> u8 {
    let pos = app
        .world()
        .get::<Position>(entity)
        .copied()
        .unwrap_or_else(Position::default);
    *pos.level()
}

/// Read a faller's current `Hp`.
pub(crate) fn hp_of(app: &App, entity: Entity) -> u16 {
    *app.world().get::<Hp>(entity).copied().unwrap_or(Hp::new(0))
}

/// Write a `SlabDestroyed` at the column cell + `level`, then drive two updates so the
/// buffered signal reaches `apply_falls` regardless of intra-frame order (buffered messages
/// persist a frame). A test-body `World` write (`bevy-traps.md` #7 carve-out).
pub(crate) fn destroy_slab_and_settle(app: &mut App, level: u8) {
    app.world_mut()
        .write_message(SlabDestroyed::new(CellLevel::new(
            column_cell(),
            Level::new(level),
        )));
    app.update();
    app.update();
}

/// The full run history of `FallOccurred` signals (from the test-local recorder).
pub(crate) fn fall_signals(app: &App) -> Vec<FallOccurred> {
    app.world()
        .get_resource::<FallLog>()
        .map(|log| log.falls.clone())
        .unwrap_or_default()
}
