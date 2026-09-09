//! Probing line of sight over the app's own grids, and the mount band a probe leans on.

use bevy::{app::App, prelude::Entity};
use gdtf_battle_sim::{
    cover::{CoverLedger, HeightBand},
    ganger::{Facing, Position, Stance},
    los::{Observer, PeekOffset, Sighted, Target, has_los},
    march::MarchGrids,
    metric::CellLevel,
    occupancy::OccupancyGrid,
    surface::SurfaceGrid,
    tuning::CombatTuning,
};

use super::reads::cover_band;

/// The band the mount seeds at `at`, or a failure saying the layout needs a High one there.
pub(crate) fn high_mount_band(app: &App, at: CellLevel) -> HeightBand {
    let seeded = cover_band(app, at);
    assert_eq!(
        seeded,
        Some(HeightBand::High),
        "PRECONDITION: the layout needs a High mount on {at:?}. A standing eye-line sails over a \
         lower band, so a sightline result below would be about the band, not about the mount",
    );
    let Some(band) = seeded else {
        unreachable!("the assertion above reads the band as Some");
    };
    band
}

/// Whether `observer` sees `target`, with no peek lean, the grid's stair offset and nobody dead.
pub(crate) fn sighted(app: &App, observer: Entity, target: Entity) -> Sighted {
    let world = app.world();
    let (Some(eye_at), Some(eye_stance), Some(eye_facing)) = (
        world.get::<Position>(observer),
        world.get::<Stance>(observer),
        world.get::<Facing>(observer),
    ) else {
        unreachable!("a spawned ganger carries Position, Stance and Facing");
    };
    let (Some(seen_at), Some(seen_stance)) =
        (world.get::<Position>(target), world.get::<Stance>(target))
    else {
        unreachable!("a spawned ganger carries Position and Stance");
    };
    let (Some(occupancy), Some(surface), Some(cover), Some(tuning)) = (
        world.get_resource::<OccupancyGrid>(),
        world.get_resource::<SurfaceGrid>(),
        world.get_resource::<CoverLedger>(),
        world.get_resource::<CombatTuning>(),
    ) else {
        unreachable!("setup inserts the grids and the tuning this probe marches over");
    };

    has_los(
        &Observer {
            position:         eye_at,
            stance:           eye_stance,
            facing:           eye_facing,
            stair_eye_offset: occupancy.stair_eye_offset_at(eye_at),
            peek_offset:      PeekOffset::default(),
        },
        &Target {
            position: seen_at,
            stance:   seen_stance,
        },
        MarchGrids {
            occupancy,
            surface,
            cover,
        },
        tuning,
        |_| false,
    )
}
