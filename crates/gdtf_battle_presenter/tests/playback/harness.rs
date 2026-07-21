//! Shared fixture for the playback suite: a bare app carrying the cursor and a hand-built
//! act log, driven one system-run at a time with a hand-advanced clock.
//!
//! NO WALL-CLOCK WAITS ANYWHERE. Every test advances [`Time`] by an exact
//! [`Duration`](std::time::Duration) and then runs the cursor once, so a pacing assertion is
//! a statement about the code rather than about how fast the machine happened to be.

use std::time::Duration;

use bevy::{app::App, ecs::system::RunSystemOnce, prelude::*};
use gdtf_battle_presenter::{
    DrawnLife, DrawnPose, DrawnPosition, DrawnVitals, PlaybackCursor, PlaybackTuning,
    advance_playback, register_playback, seed_drawn_state,
};
use gdtf_battle_sim::{
    act_log::{ActDeed, ActLog, ActProvenance, ActSeq, RecordedAct},
    acts::RoundCount,
    ganger::{Aiming, Direction, Facing, Hp, LifeState, Position, Stance, StanceKind, Tu, Wounds},
    injuries::InflictedInjuries,
    metric::{Cell, CellLevel, Level},
    weapon::ModeKind,
};

/// A ground-floor `(cell, level)` key.
pub(crate) fn ground(x: i32, y: i32) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(0))
}

/// Build the focused playback app: the cursor, the dwell table, every `Played<M>` buffer,
/// and a hand-driven [`Time`].
///
/// No render stack and no sim plugin — the cursor reads an act log this fixture builds by
/// hand, which is exactly the input it reads in production and lets a test place entries
/// precisely.
pub(crate) fn playback_app() -> App {
    let mut app = App::new();
    app.init_resource::<Time>();
    register_playback(&mut app);
    app.insert_resource(ActLog::default());
    app
}

/// Spawn a stand-in ganger with the sim components the drawn mirrors are seeded from.
pub(crate) fn spawn_ganger(app: &mut App, at: CellLevel, facing: Direction) -> Entity {
    app.world_mut()
        .spawn((
            Position::new(at),
            Facing::new(facing),
            Stance::new(StanceKind::Standing),
            Aiming::new(false),
            LifeState::Alive,
            Tu::new(10),
            Hp::new(10),
            Wounds::new(3),
            InflictedInjuries::default(),
        ))
        .id()
}

/// Insert the drawn mirrors by running the real seeding system once.
pub(crate) fn seed(app: &mut App) {
    let ran = app.world_mut().run_system_once(seed_drawn_state);
    assert!(ran.is_ok(), "the seeding system must run in the fixture");
}

/// Append one entry to the fixture's act log.
pub(crate) fn append(app: &mut App, actor: Entity, provenance: ActProvenance, deed: ActDeed) {
    let Some(mut log) = app.world_mut().get_resource_mut::<ActLog>() else {
        unreachable!("the fixture inserts an act log");
    };
    log.append(RecordedAct::new(actor, provenance, deed));
}

/// Append a reaction-fire declaration by `actor` interrupting `interrupted`.
pub(crate) fn append_reaction_fire(app: &mut App, actor: Entity, interrupted: Entity) {
    append(
        app,
        actor,
        ActProvenance::Reaction { interrupted },
        ActDeed::Fired {
            target: Some(interrupted),
            mode:   ModeKind::Single,
            rounds: RoundCount::new(1),
        },
    );
}

/// Advance the clock by `delta` and run the cursor once.
pub(crate) fn step(app: &mut App, delta: Duration) {
    app.world_mut().resource_mut::<Time>().advance_by(delta);
    let ran = app.world_mut().run_system_once(advance_playback);
    assert!(ran.is_ok(), "the playback cursor must run in the fixture");
}

/// The cursor's current position.
pub(crate) fn shown(app: &App) -> ActSeq {
    app.world().resource::<PlaybackCursor>().shown()
}

/// Whether the cursor is currently holding.
pub(crate) fn holding(app: &App) -> bool {
    app.world().resource::<PlaybackCursor>().is_holding()
}

/// The dwell table the fixture is running with.
pub(crate) fn tuning(app: &App) -> PlaybackTuning {
    app.world().resource::<PlaybackTuning>().clone()
}

/// The drawn position of `entity`, if it has one.
pub(crate) fn drawn_position(app: &App, entity: Entity) -> Option<Position> {
    app.world()
        .get::<DrawnPosition>(entity)
        .map(DrawnPosition::position)
}

/// The drawn pose of `entity`, if it has one.
pub(crate) fn drawn_pose(app: &App, entity: Entity) -> Option<DrawnPose> {
    app.world().get::<DrawnPose>(entity).copied()
}

/// The drawn life state of `entity`, if it has one.
pub(crate) fn drawn_life(app: &App, entity: Entity) -> Option<LifeState> {
    app.world().get::<DrawnLife>(entity).map(|life| **life)
}

/// The drawn hit points of `entity`, if it has one.
pub(crate) fn drawn_hp(app: &App, entity: Entity) -> Option<Hp> {
    app.world().get::<DrawnVitals>(entity).map(DrawnVitals::hp)
}

/// Whether `pose` is drawn suppressed — a named read so an assertion reads as a predicate.
pub(crate) const fn drawn_suppressed(pose: DrawnPose) -> bool {
    pose.suppressed()
}
