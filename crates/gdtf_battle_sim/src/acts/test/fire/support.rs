pub(super) use super::super::support::*;

pub(super) fn drain_shots_fired(app: &mut App) -> Vec<ShotFired> {
    app.world_mut()
        .resource_mut::<Messages<ShotFired>>()
        .drain()
        .collect()
}

pub(super) fn drain_fire_declarations(app: &mut App) -> Vec<FireDeclaration> {
    app.world_mut()
        .resource_mut::<Messages<FireDeclaration>>()
        .drain()
        .collect()
}

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

pub(super) fn place_arc_target(app: &mut App, x: i32, y: i32) -> Entity {
    let target = app.world_mut().spawn(target_bundle(30, 6)).id();
    let at = CellLevel::new(Cell::new(x, y), Level::new(0));
    if let Some(mut grid) = app.world_mut().get_resource_mut::<OccupancyGrid>() {
        grid.set_occupant(at, Some(target));
        grid.set_occupant_band(at, Some(HeightBand::High));
    }
    target
}

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

pub(super) fn turn_tu_in(app: &App) -> Option<u8> {
    app.world()
        .get_resource::<CombatTuning>()
        .map(|t| *t.turn_tu)
}
