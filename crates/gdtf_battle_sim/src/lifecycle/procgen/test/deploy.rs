//! GTW-744 — the roster DEPLOYMENT step: [`deploy_rosters`] places each roster member into
//! its side's deployment zone, deterministically by seed, on valid/standable/non-overlapping
//! cells — driven over the REAL `generate_level` pipeline (the same functions the app runs,
//! no shadow copy), plus a hand-built fail-closed pin.
//!
//! Three concerns:
//!
//! 1. **Determinism** — the SAME seed deploys an IDENTICAL `Vec<PlacedGanger>`, and DIFFERENT
//!    seeds can deploy different placements (the pin is discriminating, not vacuous).
//! 2. **Validity** (multi-seed) — every placed cell is in-bounds, not a terrain wall, distinct,
//!    inside its side's zone region, facing the zone's centre-ward direction, Standing / Alive.
//! 3. **Fail-closed** — a zone smaller than its roster returns
//!    [`PackingError::DeploymentZoneTooSmall`].

use super::emit::support::{registry_with_fill, size, terrain_defs, theme, theme_registry};
use crate::{
    ganger::{Faction, GangName, GangerName, LifeState, StanceKind},
    metric::{Cell, CellLevel},
    procgen::{
        Anchor, DeploymentZone, DeploymentZones, Footprint, PackingError, ProcgenTuning,
        RegionRect, deploy_rosters, generate_level,
    },
    rng::{BattleSeed, ProcgenRng},
    situation::{RosterMember, Situation},
};

/// A four-member roster mirroring the shipped skirmish: two player-faction (0) members and
/// two enemy-faction (1) members, in authored order.
fn skirmish_rosters() -> Vec<RosterMember> {
    let member = |gang: &str, name: &str, faction: u8| {
        RosterMember::new(
            GangName::new(gang.to_owned()),
            GangerName::new(name.to_owned()),
            Faction::new(faction),
        )
    };
    vec![
        member("gang_0", "Alex Mercer", 0),
        member("gang_0", "Kira Vann", 0),
        member("gang_1", "Vex 1", 1),
        member("gang_1", "Vex 2", 1),
    ]
}

/// Run the REAL `generate_level` pipeline for `seed` and return the surfaced deployment zones
/// paired with the generated terrain (the same inputs the app hands `deploy_rosters`).
fn generated(seed: BattleSeed) -> Option<(DeploymentZones, Situation)> {
    let theme = theme();
    let (Some(board), Some(player_fp), Some(enemy_fp)) = (size(40, 40), size(12, 12), size(12, 12))
    else {
        return None;
    };
    let prefabs = registry_with_fill(theme, player_fp, enemy_fp, &[("hall", 8, 8)])?;
    let themes = theme_registry(theme);
    let defs = terrain_defs();
    let knobs = ProcgenTuning::default();
    let mut rng = ProcgenRng::from_root(seed);
    let emitted = generate_level(&prefabs, &themes, &defs, theme, board, &mut rng, &knobs).ok()?;
    Some((emitted.zones, emitted.situation))
}

/// Concern 1 — determinism: the SAME seed deploys an IDENTICAL placement set, and at least one
/// DIFFERENT seed deploys a different one (so the pin is discriminating).
#[test]
fn deploy_is_deterministic_and_discriminating_by_seed() {
    // Fix the map (one generate_level) so we isolate the DEPLOY seed's effect on the shuffle.
    let gen_seed = BattleSeed::new(0x0744_0001);
    let Some((zones, terrain)) = generated(gen_seed) else {
        return;
    };
    let rosters = skirmish_rosters();
    let deploy =
        |seed: BattleSeed| deploy_rosters(&zones, &terrain, &rosters, Faction::new(0), seed);

    let seed = BattleSeed::new(0xD0E5_744A);
    let first = deploy(seed);
    let second = deploy(seed);
    assert!(
        first.is_ok() && second.is_ok(),
        "deploy must succeed on the 12x12 fixture zones, got {first:?} / {second:?}",
    );
    let (Ok(a), Ok(b)) = (first, second) else {
        return;
    };
    assert_eq!(
        a, b,
        "the same seed must deploy an IDENTICAL placement set (determinism)",
    );

    // A different deploy seed shuffles the zone differently — over several seeds at least one
    // must diverge (the pin is not vacuously equal for all seeds).
    let differs = [1u64, 2, 3, 5, 8, 13, 21, 34]
        .into_iter()
        .filter_map(|s| deploy(BattleSeed::new(s)).ok())
        .any(|other| other != a);
    assert!(
        differs,
        "different seeds must be able to deploy DIFFERENT placements (discriminating pin)",
    );
}

/// Concern 2 — validity (multi-seed): over several seeds every placed ganger lands on a valid
/// cell (in-bounds, not a terrain wall, distinct), inside its side's zone, facing the zone's
/// centre-ward direction, Standing / not-aiming / Alive.
#[test]
fn deployment_is_valid_across_seeds() {
    let rosters = skirmish_rosters();
    for raw in [0u64, 1, 7, 42, 1000, 65_535] {
        let seed = BattleSeed::new(raw);
        let Some((zones, terrain)) = generated(seed) else {
            continue;
        };
        let result = deploy_rosters(&zones, &terrain, &rosters, Faction::new(0), seed);
        assert!(
            result.is_ok(),
            "deploy must succeed for seed {raw}, got {result:?}"
        );
        let Ok(placed) = result else {
            continue;
        };

        assert_eq!(
            placed.len(),
            rosters.len(),
            "every roster member must be deployed (seed {raw})",
        );

        let width = i32::from(*terrain.grid_size.width());
        let height = i32::from(*terrain.grid_size.height());
        let wall_cells: Vec<CellLevel> = terrain.walls.iter().map(|w| w.at).collect();
        let mut used_cells: Vec<CellLevel> = Vec::new();
        for ganger in &placed {
            // Distinct cells (no two gangers stacked).
            assert!(
                !used_cells.contains(&ganger.at),
                "seed {raw}: deployed cell {:?} is not distinct",
                ganger.at,
            );
            used_cells.push(ganger.at);

            // In-bounds against the generated board.
            assert!(
                ganger.at.x >= 0 && ganger.at.x < width && ganger.at.y >= 0 && ganger.at.y < height,
                "seed {raw}: deployed cell {:?} is out of bounds",
                ganger.at,
            );

            // Not on a terrain wall (standable).
            assert!(
                !wall_cells.contains(&ganger.at),
                "seed {raw}: deployed cell {:?} is a terrain wall (not standable)",
                ganger.at,
            );

            // Inside its side's zone, facing that zone's centre-ward direction.
            let zone = if *ganger.faction == 0 {
                zones.player()
            } else {
                zones.enemy()
            };
            assert!(
                region_contains(zone.region(), ganger.at),
                "seed {raw}: {} deployed at {:?} outside its zone {:?}",
                *ganger.member,
                ganger.at,
                zone.region(),
            );
            assert_eq!(
                *ganger.facing,
                zone.facing(),
                "seed {raw}: {} must face its zone's centre-ward direction",
                *ganger.member,
            );

            // The deploy defaults: Standing, not aiming, Alive.
            assert_eq!(*ganger.stance, StanceKind::Standing, "seed {raw}: Standing");
            assert!(!*ganger.aiming, "seed {raw}: not aiming");
            assert_eq!(ganger.life_state, LifeState::Alive, "seed {raw}: Alive");
        }
    }
}

/// Concern 3 — fail-closed: a zone with fewer standable cells than its roster returns
/// [`PackingError::DeploymentZoneTooSmall`] naming the demand and capacity.
#[test]
fn deploy_fails_closed_when_a_zone_cannot_fit_its_roster() {
    // A 1x1 player zone (capacity 1 standable cell on empty terrain) but TWO player members.
    let zones = DeploymentZones::new(
        DeploymentZone::new(
            Anchor::BottomLeft,
            RegionRect::new(Cell::new(0, 0), Footprint::new(1, 1)),
        ),
        DeploymentZone::new(
            Anchor::TopRight,
            RegionRect::new(Cell::new(50, 50), Footprint::new(4, 4)),
        ),
    );
    let terrain = Situation::new(); // empty (60x60x8 default), so (0,0) is standable
    let rosters = vec![
        RosterMember::new(
            GangName::new("gang_0".to_owned()),
            GangerName::new("A".to_owned()),
            Faction::new(0),
        ),
        RosterMember::new(
            GangName::new("gang_0".to_owned()),
            GangerName::new("B".to_owned()),
            Faction::new(0),
        ),
    ];

    let result = deploy_rosters(
        &zones,
        &terrain,
        &rosters,
        Faction::new(0),
        BattleSeed::new(1),
    );
    assert!(
        matches!(result, Err(PackingError::DeploymentZoneTooSmall { .. })),
        "a 1x1 zone with two members must fail closed with DeploymentZoneTooSmall, got {result:?}",
    );
    let Err(PackingError::DeploymentZoneTooSmall {
        anchor,
        demand,
        capacity,
    }) = result
    else {
        return;
    };
    assert_eq!(anchor, Anchor::BottomLeft, "the offending zone's anchor");
    assert_eq!(*demand, 2, "demand names the two members");
    assert_eq!(*capacity, 1, "capacity names the single standable cell");
}

/// Whether `cell_level`'s x/y lie within `region` (a half-open `[origin, origin+extent)`
/// rectangle; the storey is ignored — deployment is on the ground plane).
fn region_contains(region: RegionRect, cell_level: CellLevel) -> bool {
    let origin = region.origin();
    let footprint = region.footprint();
    cell_level.x >= origin.x
        && cell_level.x < origin.x + footprint.width()
        && cell_level.y >= origin.y
        && cell_level.y < origin.y + footprint.height()
}
