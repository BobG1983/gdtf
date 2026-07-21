//! [`deploy_rosters`] — the GTW-744 roster deployment step: place each
//! [`RosterMember`] into its side's [`DeploymentZone`] on a valid, standable,
//! non-overlapping cell, DETERMINISTICALLY by the battle
//! [`BattleSeed`](crate::rng::BattleSeed).
//!
//! This is render-free, deterministic MODEL logic (the *behavioral* render-free constraint
//! the rest of `procgen` holds): given the two [`DeploymentZones`] the assembler surfaced,
//! the generated terrain, the roster, and a seed, it returns the placed gangers — or fails
//! closed with a typed [`PackingError::DeploymentZoneTooSmall`] if a zone cannot stand its
//! whole roster. It NEVER `unwrap`/`expect`/`panic`s.
//!
//! # Determinism
//!
//! The only RNG is a [`DeploymentRng`] derived from `seed` via
//! [`DeploymentRng::from_root`], drawn to Fisher-Yates-shuffle each zone's standable cells.
//! Sides are processed in a FIXED order (player then enemy), and the roster is filtered in
//! its authored order, so the SAME seed always produces the SAME `Vec<PlacedGanger>` (a
//! step-equivalence-style pin).
//!
//! # Validity + non-overlap
//!
//! A cell is STANDABLE only if it is in-bounds, not blocked by terrain (a wall / cover /
//! emplacement — read through the authoritative [`OccupancyGrid::is_blocked`], never
//! re-derived), not already occupied by an authored ganger, and not already assigned this
//! pass. Because the player and enemy zones are opposite prefab regions separated by the
//! inter-prefab margin,
//! their standable sets are disjoint; the running `occupied` set makes non-overlap global
//! even if a fixture authored both `gangers` and `rosters`.

use bevy::{platform::collections::HashSet, prelude::Deref};

use super::zone::{DeploymentZone, DeploymentZones};
use crate::{
    ganger::{Aiming, Facing, Faction, LifeState, Stance, StanceKind},
    level::GridSize,
    metric::{Cell, CellLevel, Level},
    occupancy::{OccupancyGrid, OccupancyInput, TerrainKind, TerrainPlacement},
    procgen::{PackingError, RosterDemand, ZoneCapacity},
    rng::{BattleSeed, DeploymentRng},
    situation::{PlacedGanger, Placement, RosterMember, Situation},
};

/// Whether a `(cell, level)` is **standable** for a deploying ganger (GTW-744) — in-bounds,
/// unblocked, and unoccupied.
///
/// A named newtype over `bool` (no-bare-types: a placement-legality verdict is a domain
/// fact, not a bare boolean), mirroring the geometry predicates
/// ([`RectNonEmpty`](crate::procgen::RectNonEmpty)). Private inner + derived [`Deref`].
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Standable(bool);

impl Standable {
    /// Build a standable verdict from its boolean state.
    #[must_use]
    pub const fn new(standable: bool) -> Self {
        Self(standable)
    }
}

/// Deploy every [`RosterMember`] into its side's [`DeploymentZone`], returning the placed
/// gangers or a fail-closed [`PackingError::DeploymentZoneTooSmall`] (GTW-744).
///
/// `player_faction` splits the roster: a member whose faction equals it deploys in the
/// player zone, every other faction in the enemy zone. Each member lands on a distinct
/// standable cell of its zone, facing the board centre (derived from the zone anchor), and
/// spawns Standing / not-aiming / Alive. Deterministic in `seed` (see the module doc).
///
/// # Errors
///
/// [`PackingError::DeploymentZoneTooSmall`] if a zone has fewer standable cells than its
/// roster has members (the fail-closed contract — no dropped or stacked members).
pub fn deploy_rosters(
    zones: &DeploymentZones,
    terrain: &Situation,
    rosters: &[RosterMember],
    player_faction: Faction,
    seed: BattleSeed,
) -> Result<Vec<PlacedGanger>, PackingError> {
    let grid = blocking_grid(terrain);
    let grid_size = terrain.grid_size;
    // Seed the occupied set with any authored ganger cells, so a fixture that authors BOTH
    // `gangers` and `rosters` still gets globally non-overlapping placements.
    let mut occupied: HashSet<CellLevel> = terrain.gangers.iter().map(|g| g.at).collect();
    let mut rng = DeploymentRng::from_root(seed);
    let mut placed: Vec<PlacedGanger> = Vec::new();

    // FIXED order: player side then enemy side (deterministic stream consumption). The
    // roster is filtered in its authored order for each side.
    let player_members: Vec<&RosterMember> = rosters
        .iter()
        .filter(|m| m.faction == player_faction)
        .collect();
    deploy_side(
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

/// Place `members` into one `zone`: collect its standable cells, fail closed if there are
/// too few, else shuffle them deterministically and assign one per member (GTW-744).
fn deploy_side(
    zone: DeploymentZone,
    members: &[&RosterMember],
    grid: &OccupancyGrid,
    grid_size: GridSize,
    occupied: &mut HashSet<CellLevel>,
    rng: &mut DeploymentRng,
    placed: &mut Vec<PlacedGanger>,
) -> Result<(), PackingError> {
    if members.is_empty() {
        return Ok(());
    }

    // Every standable cell of the zone at ground level, in row-major order.
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

    // Fisher-Yates shuffle in place (the `0..=i` range is non-empty by construction, so the
    // draw never panics) — spreads members across the zone deterministically by seed.
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

/// Whether a `(cell, level)` is standable: in-bounds against `grid_size`, not blocked by
/// terrain (via the authoritative [`OccupancyGrid::is_blocked`]), and not in the `occupied`
/// set (GTW-744).
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

/// Build a blocking-only [`OccupancyGrid`] from the terrain's walls + scatter — every such
/// cell marked [`TerrainKind::Wall`] so [`OccupancyGrid::is_blocked`] answers "a ganger
/// cannot stand here" over the authoritative blocking logic (GTW-744).
///
/// Both `Wall` and `Cover` block a slot, so marking every walls/scatter cell `Wall` is a
/// sound stand-test (a slab, by contrast, leaves its slot `Open` — a floor a ganger stands
/// on). This reuses the real blocking query rather than re-deriving it.
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
