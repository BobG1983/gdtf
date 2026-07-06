//! Shared fire-test drains + arc fixtures — re-exports the outer acts test
//! support, so each concern file reaches everything via `use super::support::*;`.

pub(super) use super::super::support::*;

/// Drain the buffered [`ShotFired`] messages emitted this run, in order. `drain`
/// empties the buffer, so the test runs ONE `update()` then probes (the
/// `drain_battle_ready` precedent).
pub(super) fn drain_shots_fired(app: &mut App) -> Vec<ShotFired> {
    app.world_mut()
        .resource_mut::<Messages<ShotFired>>()
        .drain()
        .collect()
}

/// Drain the buffered [`FireDeclaration`] combat-log messages emitted this run (GTW-328).
pub(super) fn drain_fire_declarations(app: &mut App) -> Vec<FireDeclaration> {
    app.world_mut()
        .resource_mut::<Messages<FireDeclaration>>()
        .drain()
        .collect()
}

/// Spawn an armed shooter facing `facing` at `(x, y, 0)` with a given starting `tu`
/// pool — the GTW-242 fixture (reuses `spawn_shooter`, then overrides the facing + the
/// pool so a test can place the shooter at the boundary of an affordability gate). The
/// shooter is NOT aiming (hip-fire) so the fire cost has no aim premium muddying the
/// relation. Returns the shooter [`Entity`].
pub(super) fn spawn_arc_shooter(
    world: &mut World,
    x: i32,
    y: i32,
    facing: Direction,
    tu: u8,
) -> Entity {
    let mode = single_mode(0.2, 1);
    let shooter = spawn_shooter(world, x, y, mode, false);
    if let Ok(mut entity) = world.get_entity_mut(shooter) {
        entity.insert((Facing::new(facing), Tu::new(tu)));
    }
    shooter
}

/// Place a HIGH-band ganger target at `(x, y, 0)` in the occupancy grid + spawn its
/// battle-state bundle — the GTW-242 target fixture (mirrors `fire_scenario`).
pub(super) fn place_arc_target(app: &mut App, x: i32, y: i32) -> Entity {
    let target = app.world_mut().spawn(target_bundle(30, 6)).id();
    let at = CellLevel::new(Cell::new(x, y), Level::new(0));
    if let Some(mut grid) = app.world_mut().get_resource_mut::<OccupancyGrid>() {
        grid.set_occupant(at, Some(target));
        grid.set_occupant_band(at, Some(HeightBand::High));
    }
    target
}

/// The fire-TU cost a hip-fired `single_mode(0.2, 1)` shot charges under the app's
/// current [`CombatTuning`] (the same `mode_tu_cost` source `fire()` debits) — read from
/// the resource so the relation tracks the tunable value, never a pinned magnitude.
pub(super) fn fire_cost_in(app: &App) -> Option<u8> {
    let tuning = app.world().get_resource::<CombatTuning>()?;
    let mode = single_mode(0.2, 1);
    Some(*mode_tu_cost(
        &mode,
        &TuMax::new(100),
        &Aiming::new(false),
        tuning,
    ))
}

/// The per-45°-step turn cost off the app's current [`CombatTuning`].
pub(super) fn turn_tu_in(app: &App) -> Option<u8> {
    app.world()
        .get_resource::<CombatTuning>()
        .map(|t| *t.turn_tu)
}
