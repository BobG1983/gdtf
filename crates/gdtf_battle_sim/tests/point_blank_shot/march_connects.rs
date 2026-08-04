use bevy::{math::Vec3, prelude::World};
use gdtf_battle_sim::{
    central_axis::{climb_aim_dir, muzzle_position, target_aim_point},
    cone::PriorShots,
    cover::{CoverLedger, HeightBand},
    ganger::Facing,
    march::{MarchDir, MarchKind, march_vector},
    prelude::{Cell, CellLevel, Direction, Level, OccupancyGrid, Position, Stance, StanceKind},
    stability::RecoilGrowth,
    surface::SurfaceGrid,
    tuning::{CombatTuning, RecoilClimb},
};

use super::harness::*;

const fn stance_band(stance: StanceKind) -> HeightBand {
    match stance {
        StanceKind::Standing => HeightBand::High,
        StanceKind::Crouching => HeightBand::Mid,
        StanceKind::Prone => HeightBand::Low,
    }
}

fn point_blank_march_hits(
    facing: Direction,
    enemy_cell: CellLevel,
    shooter_stance: StanceKind,
    target_stance: StanceKind,
) -> bool {
    let tuning = CombatTuning::default();

    let mut probe_world = World::new();
    let enemy = probe_world.spawn_empty().id();
    let mut occupancy = OccupancyGrid::new();
    occupancy.set_occupant(enemy_cell, Some(enemy));
    occupancy.set_occupant_band(enemy_cell, Some(stance_band(target_stance)));
    let surface = SurfaceGrid::new();
    let cover = CoverLedger::new();

    let shooter_pos = Position::new(shooter_cell());
    let muzzle = muzzle_position(
        shooter_pos,
        Facing::new(facing),
        Stance::new(shooter_stance),
        &tuning,
    );
    let aim = target_aim_point(
        Position::new(enemy_cell),
        Stance::new(target_stance),
        Some(stance_band(target_stance)),
        &tuning,
    );
    let dir: Vec3 = climb_aim_dir(
        muzzle,
        aim,
        PriorShots::first(),
        RecoilClimb::new(0.0),
        RecoilGrowth::new(0.0),
    )
    .vec();

    let result = march_vector(
        muzzle,
        MarchDir::new(dir),
        &occupancy,
        &surface,
        &cover,
        &tuning,
        *shooter_pos,
        |_| false,
    );
    result.kind == MarchKind::Ganger(enemy)
}

#[test]
fn point_blank_march_connects_for_every_facing_and_stance() {
    let facings = [
        (Direction::North, Cell::new(5, 5)),
        (Direction::NorthEast, Cell::new(6, 5)),
        (Direction::East, Cell::new(6, 6)),
        (Direction::SouthEast, Cell::new(6, 7)),
        (Direction::South, Cell::new(5, 7)),
        (Direction::SouthWest, Cell::new(4, 7)),
        (Direction::West, Cell::new(4, 6)),
        (Direction::NorthWest, Cell::new(4, 5)),
    ];
    let stances = [
        StanceKind::Standing,
        StanceKind::Crouching,
        StanceKind::Prone,
    ];

    let mut misses: Vec<String> = Vec::new();
    for (facing, enemy_xy) in facings {
        let enemy_cell = CellLevel::new(enemy_xy, Level::new(0));
        for shooter_stance in stances {
            for target_stance in stances {
                if !point_blank_march_hits(facing, enemy_cell, shooter_stance, target_stance) {
                    misses.push(format!(
                        "{facing:?} shooter={shooter_stance:?} target={target_stance:?}"
                    ));
                }
            }
        }
    }

    assert!(
        misses.is_empty(),
        "every point-blank shot must connect; {} missed: {misses:?}",
        misses.len(),
    );
}
