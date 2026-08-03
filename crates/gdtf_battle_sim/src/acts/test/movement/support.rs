pub(super) use super::super::support::*;

pub(super) fn spawn_move_actor(world: &mut World, x: i32, y: i32, tu: u8) -> Entity {
    world
        .spawn((
            Position::new(CellLevel::new(Cell::new(x, y), Level::new(0))),
            Tu::new(tu),
            LifeState::Alive,
            Faction::new(TEST_PLAYER_GANG),
        ))
        .id()
}

pub(super) fn drain_movements(app: &mut App) -> Vec<MovementOccurred> {
    app.world_mut()
        .resource_mut::<Messages<MovementOccurred>>()
        .drain()
        .collect()
}

pub(super) fn spawn_move_actor_of_gang(
    world: &mut World,
    x: i32,
    y: i32,
    tu: u8,
    gang: u8,
) -> Entity {
    world
        .spawn((
            Position::new(CellLevel::new(Cell::new(x, y), Level::new(0))),
            Tu::new(tu),
            LifeState::Alive,
            Faction::new(gang),
        ))
        .id()
}

pub(super) fn drain_rejects(app: &mut App) -> Vec<MoveRejected> {
    app.world_mut()
        .resource_mut::<Messages<MoveRejected>>()
        .drain()
        .collect()
}

pub(super) fn author_cover(app: &mut App, x: i32, y: i32) {
    let at = CellLevel::new(Cell::new(x, y), Level::new(0));
    let entry = CoverEntry::seeded(
        CoverHp::new(10),
        HeightBand::Mid,
        ArmorProtection::new(0),
        ArmorHardness::new(0),
    );
    if let Some(mut ledger) = app.world_mut().get_resource_mut::<CoverLedger>() {
        ledger.insert(at, entry);
    }
}

pub(super) fn suppress_from(app: &mut App, actor: Entity, sx: i32, sy: i32) {
    let from = SuppressorCell::new(CellLevel::new(Cell::new(sx, sy), Level::new(0)));
    if let Ok(mut entity) = app.world_mut().get_entity_mut(actor) {
        entity.insert(Suppressed::new(from));
    }
}
