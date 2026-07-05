//! Shared mover fixtures + message drains — re-exports the outer acts test
//! support, so each concern file reaches everything via `use super::support::*;`.

pub(super) use super::super::support::*;

/// Spawn a move-capable actor ([`Position`] / [`Tu`] / [`LifeState::Alive`] /
/// [`Faction`]) at `(x, y, 0)`.
///
/// GTW-354: the move dispatch fetches `&Faction` (to classify route occupants relative to
/// the mover) and runs `find_path` — so a move actor carries the player gang
/// ([`TEST_PLAYER_GANG`]) so it is found by the actor query and so its own-squad relation
/// resolves.
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

/// Drain the buffered [`MovementOccurred`] combat-log messages emitted this run (GTW-328).
pub(super) fn drain_movements(app: &mut App) -> Vec<MovementOccurred> {
    app.world_mut()
        .resource_mut::<Messages<MovementOccurred>>()
        .drain()
        .collect()
}

/// Spawn a move-capable actor of `gang` (its own faction) at `(x, y, 0)` — the
/// faction-parameterized sibling of [`spawn_move_actor`] (which always uses
/// [`TEST_PLAYER_GANG`]). Used by the GTW-459 regression test to put an ENEMY mover
/// (a non-player gang) on the field alongside a player-gang downed body.
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

/// Drain the buffered [`MoveRejected`] signals emitted this run (GTW-354).
pub(super) fn drain_rejects(app: &mut App) -> Vec<MoveRejected> {
    app.world_mut()
        .resource_mut::<Messages<MoveRejected>>()
        .drain()
        .collect()
}

/// Register a piece of MID-band cover in the [`CoverLedger`] at `(x, y, 0)` so a suppressed
/// mover can end BEHIND it (GTW-537). Arbitrary (not shipped) HP / armor magnitudes — the gate
/// only cares that the peek returns `Some`, never the numbers.
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

/// Mark `actor` [`Suppressed`] anchored to a suppressor at `(sx, sy, 0)` (GTW-537) — the
/// directional anchor the movement gate measures "farther from" + "behind cover relative to".
pub(super) fn suppress_from(app: &mut App, actor: Entity, sx: i32, sy: i32) {
    let from = SuppressorCell::new(CellLevel::new(Cell::new(sx, sy), Level::new(0)));
    if let Ok(mut entity) = app.world_mut().get_entity_mut(actor) {
        entity.insert(Suppressed::new(from));
    }
}
