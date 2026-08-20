pub(super) use super::super::support::*;
pub(super) use crate::{
    acts::{DismountSurcharge, SightWorld},
    clearance::silhouette_band,
    surface::SurfaceGrid,
};

/// A mover on foot pays no seat's exit on top of its route.
pub(super) const AFOOT: DismountSurcharge = DismountSurcharge::NONE;

/// The pose a mover holds unless a case poses it otherwise.
pub(super) const STANDING: Stance = Stance::new(StanceKind::Standing);
pub(super) const NORTH: Facing = Facing::new(Direction::North);

/// The mover in a fixture with no world behind it, so no body on the grid answers to it.
pub(super) const UNSPAWNED_MOVER: Entity = Entity::PLACEHOLDER;

/// Terrain with nothing standing on it, so only what a case seeds can break a ray.
pub(super) struct BareTerrain {
    occupancy: OccupancyGrid,
    surface:   SurfaceGrid,
    tuning:    CombatTuning,
}

impl BareTerrain {
    pub(super) fn new() -> Self {
        Self {
            occupancy: OccupancyGrid::new(),
            surface:   SurfaceGrid::new(),
            tuning:    CombatTuning::default(),
        }
    }

    /// Stand a live body at `at`, which a ray reads as a blocker like any other ganger.
    pub(super) fn stand(&mut self, at: CellLevel, who: Entity) {
        self.occupancy.set_occupant(at, Some(who));
        self.occupancy
            .set_occupant_band(at, Some(silhouette_band(StanceKind::Standing)));
    }

    pub(super) fn sight(&self) -> SightWorld<'_, impl Fn(Entity) -> bool> {
        SightWorld::new(&self.occupancy, &self.surface, &self.tuning, |_| false)
    }
}

pub(super) fn spawn_move_actor(world: &mut World, x: i32, y: i32, tu: u8) -> Entity {
    spawn_move_actor_on(world, x, y, Level::new(0), tu)
}

pub(super) fn spawn_move_actor_on(
    world: &mut World,
    x: i32,
    y: i32,
    storey: Level,
    tu: u8,
) -> Entity {
    world
        .spawn((
            Position::new(CellLevel::new(Cell::new(x, y), storey)),
            Tu::new(tu),
            LifeState::Alive,
            Faction::new(TEST_PLAYER_GANG),
            Stance::new(StanceKind::Standing),
            Facing::new(Direction::North),
        ))
        .id()
}

pub(super) fn drain_movements(app: &mut App) -> Vec<MovementOccurred> {
    app.world_mut()
        .resource_mut::<Messages<MovementOccurred>>()
        .drain()
        .collect()
}

pub(super) fn drain_completed_moves(app: &mut App) -> Vec<MoveCompleted> {
    app.world_mut()
        .resource_mut::<Messages<MoveCompleted>>()
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
            Stance::new(StanceKind::Standing),
            Facing::new(Direction::North),
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
    author_cover_on(app, x, y, Level::new(0));
}

pub(super) fn author_cover_on(app: &mut App, x: i32, y: i32, storey: Level) {
    let at = CellLevel::new(Cell::new(x, y), storey);
    let entry = CoverEntry::seeded(
        CoverHp::new(10),
        HeightBand::Mid,
        ArmorProtection::new(0),
        ArmorHardness::new(0),
        TerrainPieceKind::Cover,
    );
    if let Some(mut ledger) = app.world_mut().get_resource_mut::<CoverLedger>() {
        ledger.insert(at, entry);
    }
}

/// Seed cover tall enough to stop a ray at this cell, whatever stance either end holds.
pub(super) fn author_wall(app: &mut App, x: i32, y: i32) {
    let at = CellLevel::new(Cell::new(x, y), Level::new(0));
    let entry = CoverEntry::seeded(
        CoverHp::new(10),
        HeightBand::High,
        ArmorProtection::new(0),
        ArmorHardness::new(0),
        TerrainPieceKind::Wall,
    );
    if let Some(mut ledger) = app.world_mut().get_resource_mut::<CoverLedger>() {
        ledger.insert(at, entry);
    }
}

pub(super) fn suppress_from(app: &mut App, actor: Entity, sx: i32, sy: i32) {
    suppress_from_on(app, actor, sx, sy, Level::new(0));
}

pub(super) fn suppress_from_on(app: &mut App, actor: Entity, sx: i32, sy: i32, storey: Level) {
    let from = SuppressorCell::new(CellLevel::new(Cell::new(sx, sy), storey));
    if let Ok(mut entity) = app.world_mut().get_entity_mut(actor) {
        entity.insert(Suppressed::new(from));
    }
}
