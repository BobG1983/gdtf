//! Shared GTW-525 shove fixture: the focused shove app, the ganger spawner, the settle
//! driver, the shipped-tuning parse, and the pool accessors.

use bevy::{
    app::{App, Update},
    prelude::{Entity, MessageReader, ResMut, Resource, World},
};
use gdtf_battle_sim::{
    Cell, CellLevel, CombatTuning, Faction, FallOccurred, Hp, Level, OccupancyMaintenancePlugin,
    Position, ShoveRequested, StanceKind, Tu, Wounds,
    falls::FallsPlugin,
    test_support::{GangerEntityBuilder, SimAppBuilder},
};

/// An arbitrary fixed seed — determinism is the property, the value is irrelevant.
pub(crate) const SEED: u64 = 0x0525_5405_DEAD_BEEF;

/// The SHIPPED combat tuning parsed from the real `assets/core_tuning/combat.tuning.ron` (the
/// realistic severity edges + the real `shove_tu` leaf). Falls back to the default on a parse
/// failure (defensive; the tuning-test suite guards the parse-OK path).
pub(crate) fn shipped_tuning() -> CombatTuning {
    const SHIPPED: &str = include_str!("../../../../assets/core_tuning/combat.tuning.ron");
    ron::from_str::<CombatTuning>(SHIPPED).unwrap_or_default()
}

// ── Test-local signal recorders (the gtw523 / gtw507 idiom) ────────────────────

/// Every `FallOccurred` observed across the run.
#[derive(Resource, Default)]
pub(crate) struct FallLog {
    /// One entry per `FallOccurred` emitted.
    pub(crate) falls: Vec<FallOccurred>,
}

/// Drain `FallOccurred` into the recorder.
pub(crate) fn record_falls(mut reader: MessageReader<FallOccurred>, mut log: ResMut<FallLog>) {
    for signal in reader.read() {
        log.falls.push(*signal);
    }
}

/// Build the focused real-path app: `MinimalPlugins` + `SimActsPlugin` (which owns
/// `dispatch_shove`) + `OccupancyMaintenancePlugin` + `FallsPlugin` (the `FallOccurred` buffer).
/// Inserts the sim resources the shove + its fall read. The `Simulate` band here has NO
/// `BattleInProgress` gate (that lives in `BattleSimPlugin`), so `dispatch_shove` runs
/// unconditionally over the inserted resources — the gtw523 focused-harness pattern.
pub(crate) fn shove_app() -> App {
    // The canonical `with_acts` litany (GTW-576) seeds the grids + five RNG streams + empty
    // injury content the whole Simulate band validates against. Overlaid here: the SHIPPED
    // tuning (real severity edges + real shove_tu) so the shove TU spend + any fall bucket
    // are realistic (a parse failure falls back to the default — the test still runs).
    let mut app = SimAppBuilder::new()
        .with_seed(SEED)
        .with_acts()
        .with_tuning(shipped_tuning())
        .build();
    app.add_plugins(OccupancyMaintenancePlugin)
        .add_plugins(FallsPlugin);
    app.init_resource::<FallLog>();
    app.add_systems(Update, record_falls);
    app
}

/// Spawn a standing, alive ganger of `faction` at `(cell, level)` with the shove-relevant
/// component set (a large HP/Wounds pool so a fall wounds but rarely kills — a clean HP read).
/// Bare flesh (no worn armor). Returns its entity.
pub(crate) fn shove_ganger(world: &mut World, at: CellLevel, faction: u8) -> Entity {
    GangerEntityBuilder::new()
        .at(at)
        .stance(StanceKind::Standing)
        .faction(Faction::new(faction))
        .tu(100)
        .tu_max(100)
        .combat_vitals(1000, 200)
        .toughness(0.0)
        .luck(0.0)
        .spawn(world)
}

/// Read a ganger's current `(cell, level)`.
pub(crate) fn pos_of(app: &App, entity: Entity) -> Option<CellLevel> {
    app.world().get::<Position>(entity).map(|p| **p)
}

/// Read a ganger's current `Hp`.
pub(crate) fn hp_of(app: &App, entity: Entity) -> u16 {
    *app.world().get::<Hp>(entity).copied().unwrap_or(Hp::new(0))
}

/// Read a ganger's current `Tu`.
pub(crate) fn tu_of(app: &App, entity: Entity) -> u8 {
    *app.world().get::<Tu>(entity).copied().unwrap_or(Tu::new(0))
}

/// Read a ganger's current `Wounds`.
pub(crate) fn wounds_of(app: &App, entity: Entity) -> u8 {
    *app.world()
        .get::<Wounds>(entity)
        .copied()
        .unwrap_or(Wounds::new(0))
}

/// Write a deliberate `ShoveRequested` and settle two updates so the buffered message reaches
/// `dispatch_shove` regardless of intra-frame order (buffered messages persist a frame). A
/// test-body `World` write (`bevy-traps.md` #7 carve-out).
pub(crate) fn shove_and_settle(app: &mut App, shover: Entity, target: Entity) {
    app.world_mut()
        .write_message(ShoveRequested::new(shover, target));
    app.update();
    app.update();
}

/// A ground-floor `(x, y)` key.
pub(crate) fn ground(x: i32, y: i32) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(0))
}

/// An upper-storey `(x, y, level)` key.
pub(crate) fn upper(x: i32, y: i32, level: u8) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(level))
}
