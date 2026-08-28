//! A crouching gunner on a mount is stopped-at by the mount's own cover, from any bearing.

use bevy::{app::App, math::Vec3, prelude::Entity};
use gdtf_battle_sim::{
    cover::{CoverEntry, CoverLedger},
    def::rotated_entry_sides,
    ganger::Direction,
    march::{MarchDir, MarchGrids, MarchKind, MarchResult, march_vector},
    metric::{CellLevel, SimPos, cell_center},
    occupancy::OccupancyGrid,
    situation::CoverSpawn,
    surface::SurfaceGrid,
    terrain::{
        emplacement::{EmplacementEntrySides, EmplacementFacing},
        facing::TerrainFacing,
    },
    test_support::{SituationBuilder, test_terrain_registry},
    tuning::CombatTuning,
};

use super::harness::*;

/// The seed both cases run on, so the two calls differ only in the shooter's cell.
const SEED: u64 = 0x5543_1184;

/// The facing the mount is placed at, which turns its one authored side onto the gunner.
const SEAT_FACING: TerrainFacing = TerrainFacing::West;

/// The side the authored side lands on once placed, whose step reaches [`gunner_cell`].
const ENTRY_SIDE: TerrainFacing = TerrainFacing::West;

/// The cell the mount stands on.
fn mount_cell() -> CellLevel {
    ground(6, 5)
}

/// The cell its gunner enters from, one step west of the mount.
fn gunner_cell() -> CellLevel {
    ground(5, 5)
}

/// A mounted gunner, the round fired at its mount, and what the ledger holds there.
struct MountedShot {
    /// The app the seeded components are read back out of.
    app:         App,
    /// The emplacement carrying the seeded sides and facing.
    emplacement: Entity,
    /// The mount's own ledger entry at [`mount_cell`].
    entry:       CoverEntry,
    /// What the round met on its way to the mount.
    result:      MarchResult,
}

/// The registry `battle_app` inserts, with the one-sided mount added to it.
fn terrain_with_the_one_sided_mount(app: &mut App) {
    let mut terrain = test_terrain_registry();
    terrain.insert(ONE_SIDED, one_sided_emplacement());
    app.insert_resource(terrain);
}

/// The mount's rotated entry sides, read off the seeded entity.
fn rotated_sides(shot: &MountedShot) -> Vec<TerrainFacing> {
    let world = shot.app.world();
    let Some(sides) = world.get::<EmplacementEntrySides>(shot.emplacement) else {
        unreachable!("a seeded emplacement carries EmplacementEntrySides");
    };
    let Some(facing) = world.get::<EmplacementFacing>(shot.emplacement) else {
        unreachable!("a seeded emplacement carries EmplacementFacing");
    };
    rotated_entry_sides(sides, **facing)
}

/// The mount's own ledger entry, copied out so the borrow on the world ends here.
fn mount_entry(app: &App) -> CoverEntry {
    let Some(ledger) = app.world().get_resource::<CoverLedger>() else {
        unreachable!("setup inserts the CoverLedger the mount's entry is seeded into");
    };
    let Some(entry) = ledger.peek(&mount_cell()) else {
        unreachable!(
            "the cover ledger holds no entry at {:?}. No mount was seeded there",
            mount_cell(),
        );
    };
    *entry
}

/// March a `High` round from `shooter` along row 5 toward the mount, over the app's own grids.
/// The direction is the sign of the x-offset from the shooter to the mount.
fn round_into_the_mount(app: &App, shooter: CellLevel) -> MarchResult {
    let world = app.world();
    let Some(tuning) = world.get_resource::<CombatTuning>() else {
        unreachable!("battle_app inserts the CombatTuning the muzzle height is read from");
    };
    let Some(occupancy) = world.get_resource::<OccupancyGrid>() else {
        unreachable!("setup inserts the OccupancyGrid. A default one holds no mounted gunner");
    };
    let Some(surface) = world.get_resource::<SurfaceGrid>() else {
        unreachable!("setup inserts the SurfaceGrid. A default one turns the march into a Miss");
    };
    let Some(cover) = world.get_resource::<CoverLedger>() else {
        unreachable!("setup inserts the CoverLedger. A default one holds no mount entry");
    };

    let (from_cell, from_level) = shooter.split();
    let center = cell_center(from_cell, from_level);
    let above_floor = f32::midpoint(*tuning.projectile_band_edges.mid_high, 1.0);
    let muzzle = SimPos::new(center.x, center.y, center.z + above_floor);
    let toward = if mount_cell().cell().x > from_cell.x {
        1.0
    } else {
        -1.0
    };

    march_vector(
        muzzle,
        MarchDir::new(Vec3::new(toward, 0.0, 0.0)),
        MarchGrids {
            occupancy,
            surface,
            cover,
        },
        tuning,
        shooter,
        |_| false,
    )
}

/// Seat a crouching gunner on the one-sided mount, then fire a `High` round at it from `shooter`.
fn mounted_shot(shooter: CellLevel) -> MountedShot {
    let (mut app, seed) = battle_app(SEED);
    terrain_with_the_one_sided_mount(&mut app);

    let situation = SituationBuilder::new()
        .with_gangers([crouching_player_at(gunner_cell(), Direction::East)])
        .with_scatter(CoverSpawn::new(mount_cell(), ONE_SIDED, SEAT_FACING))
        .build_with_gangs();
    drive_setup(&mut app, seed, situation);

    let emplacement = seated_emplacement(&mut app, mount_cell());
    let actor = player_on(&mut app, gunner_cell());
    mount(&mut app, actor, emplacement);

    assert_eq!(
        pos_of(&app, actor),
        Some(mount_cell()),
        "PRECONDITION: the enter was refused. The gunner stands on {:?} rather than on the \
         mount, so the round below meets no mounted ganger at all",
        pos_of(&app, actor),
    );
    assert_eq!(
        grid_occupant(&app, gunner_cell()),
        None,
        "PRECONDITION: the enter was refused. The occupancy grid still holds the gunner on \
         {:?}, the cell it was to enter from",
        gunner_cell(),
    );

    let entry = mount_entry(&app);
    let result = round_into_the_mount(&app, shooter);
    MountedShot {
        app,
        emplacement,
        entry,
        result,
    }
}

#[test]
fn a_high_round_at_a_crouching_mounted_gunner_stops_at_the_mounts_cover() {
    let shot = mounted_shot(ground(2, 5));

    assert_eq!(
        shot.result.kind,
        MarchKind::Cover(shot.entry),
        "a High round clears the crouching gunner at Mid and falls through to the mount's own \
         High cover entry at the same cell; it came back {:?}",
        shot.result.kind,
    );
    assert_eq!(
        shot.result.at,
        mount_cell(),
        "the round stops on the mount's cell {:?}, not on {:?}",
        mount_cell(),
        shot.result.at,
    );
}

#[test]
fn the_mounts_cover_stops_the_round_through_its_entry_side_and_through_its_back() {
    let cases = [
        (
            "from the west, through the mount's own entry side",
            mounted_shot(ground(2, 5)),
        ),
        (
            "from the east, through a side the mount cannot be entered from",
            mounted_shot(ground(10, 5)),
        ),
    ];

    for (bearing, shot) in &cases {
        let sides = rotated_sides(shot);
        assert_eq!(
            sides,
            vec![ENTRY_SIDE],
            "PRECONDITION: the mount must turn its one authored side onto {ENTRY_SIDE:?}, the \
             side whose step reaches {:?}; it holds {} rotated side(s), {sides:?}",
            gunner_cell(),
            sides.len(),
        );
        assert_eq!(
            shot.result.kind,
            MarchKind::Cover(shot.entry),
            "a round {bearing} stops at the mount's own cover entry. Cover is keyed by cell and \
             carries no bearing; it came back {:?}",
            shot.result.kind,
        );
        assert_eq!(
            shot.result.at,
            mount_cell(),
            "a round {bearing} stops on the mount's cell {:?}, not on {:?}",
            mount_cell(),
            shot.result.at,
        );
    }
}
