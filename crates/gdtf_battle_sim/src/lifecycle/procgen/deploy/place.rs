//! Place roster members into deployment zones.

use bevy::{platform::collections::HashSet, prelude::Deref};

use super::zone::{DeploymentZone, DeploymentZones};
use crate::{
    ganger::{Aiming, Facing, Faction, LifeState, Stance, StanceKind},
    level::{GridSize, SpawnRole},
    metric::{Cell, CellLevel, Level},
    occupancy::{OccupancyGrid, OccupancyInput, TerrainKind, TerrainPlacement},
    procgen::{PackingError, RosterDemand, ZoneCapacity},
    rng::{BattleSeed, DeploymentRng},
    situation::{PlacedGanger, Placement, RosterMember, Situation},
};

/// Whether a cell can stand a ganger.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Standable(bool);

impl Standable {
    /// Wrap the flag.
    #[must_use]
    pub const fn new(standable: bool) -> Self {
        Self(standable)
    }
}

/// Place roster members into player and enemy deployment zones.
///
/// # Errors
///
/// Returns [`PackingError::EmptySide`] when a side has no roster members.
/// Returns [`PackingError::DeploymentZoneTooSmall`] when a zone has fewer standable cells than roster members for that side.
pub fn deploy_rosters(
    zones: &DeploymentZones,
    terrain: &Situation,
    rosters: &[RosterMember],
    player_faction: Faction,
    seed: BattleSeed,
) -> Result<Vec<PlacedGanger>, PackingError> {
    let grid = blocking_grid(terrain);
    let grid_size = terrain.grid_size;
    let mut occupied: HashSet<CellLevel> = terrain.gangers.iter().map(|g| g.at).collect();
    let mut rng = DeploymentRng::from_root(seed);
    let mut placed: Vec<PlacedGanger> = Vec::new();

    let player_members: Vec<&RosterMember> = rosters
        .iter()
        .filter(|m| m.faction == player_faction)
        .collect();
    deploy_side(
        SpawnRole::Player,
        zones.player(),
        &player_members,
        &grid,
        grid_size,
        &mut occupied,
        &mut rng,
        &mut placed,
    )?;
    let enemy_members: Vec<&RosterMember> = rosters
        .iter()
        .filter(|m| m.faction != player_faction)
        .collect();
    deploy_side(
        SpawnRole::Enemy,
        zones.enemy(),
        &enemy_members,
        &grid,
        grid_size,
        &mut occupied,
        &mut rng,
        &mut placed,
    )?;

    Ok(placed)
}

fn deploy_side(
    side: SpawnRole,
    zone: DeploymentZone,
    members: &[&RosterMember],
    grid: &OccupancyGrid,
    grid_size: GridSize,
    occupied: &mut HashSet<CellLevel>,
    rng: &mut DeploymentRng,
    placed: &mut Vec<PlacedGanger>,
) -> Result<(), PackingError> {
    if members.is_empty() {
        return Err(PackingError::EmptySide { side });
    }

    let origin = zone.region().origin();
    let footprint = zone.region().footprint();
    let mut standable: Vec<CellLevel> = Vec::new();
    for dy in 0..footprint.height() {
        for dx in 0..footprint.width() {
            let cell_level = CellLevel::new(Cell::new(origin.x + dx, origin.y + dy), Level::new(0));
            if *is_standable(cell_level, grid, grid_size, occupied) {
                standable.push(cell_level);
            }
        }
    }

    if standable.len() < members.len() {
        return Err(PackingError::DeploymentZoneTooSmall {
            anchor:   zone.anchor(),
            demand:   RosterDemand::new(members.len()),
            capacity: ZoneCapacity::new(standable.len()),
        });
    }

    for i in (1..standable.len()).rev() {
        let j: usize = rng.random_range(0..=i);
        standable.swap(i, j);
    }

    let facing = zone.facing();
    for (member, &cell_level) in members.iter().zip(standable.iter()) {
        occupied.insert(cell_level);
        placed.push(PlacedGanger::new(
            member.gang.clone(),
            member.member.clone(),
            Placement::new(
                cell_level,
                member.faction,
                Facing::new(facing),
                Stance::new(StanceKind::Standing),
                Aiming::new(false),
                LifeState::Alive,
            ),
        ));
    }
    Ok(())
}

fn is_standable(
    cell_level: CellLevel,
    grid: &OccupancyGrid,
    grid_size: GridSize,
    occupied: &HashSet<CellLevel>,
) -> Standable {
    let width = i32::from(*grid_size.width());
    let height = i32::from(*grid_size.height());
    let in_bounds =
        cell_level.x >= 0 && cell_level.x < width && cell_level.y >= 0 && cell_level.y < height;
    Standable::new(in_bounds && !*grid.is_blocked(&cell_level) && !occupied.contains(&cell_level))
}

fn blocking_grid(terrain: &Situation) -> OccupancyGrid {
    let terrain_placements: Vec<TerrainPlacement> = terrain
        .walls
        .iter()
        .chain(terrain.scatter.iter())
        .map(|cover| TerrainPlacement::new(cover.at, TerrainKind::Wall))
        .collect();
    let input = OccupancyInput {
        terrain:   terrain_placements,
        occupants: Vec::new(),
    };
    OccupancyGrid::build_from_occupancy_input(&input, &HashSet::default())
}
