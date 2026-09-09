use std::time::Duration;

use bevy::{
    app::App,
    ecs::{message::Messages, system::RunSystemOnce},
    prelude::*,
};
use gdtf_battle_presenter::{
    DrawnPose, DrawnPosition, PlaybackCursor, PlaybackTuning, Played, advance_playback,
    register_playback, seed_drawn_state,
};
use gdtf_battle_sim::{
    act_log::{ActDeed, ActLog, ActProvenance, ActSeq, ActWitnesses, RecordedAct},
    acts::RoundCount,
    ganger::{Aiming, Direction, Facing, Hp, LifeState, Position, Stance, StanceKind, Tu, Wounds},
    injuries::InflictedInjuries,
    metric::{Cell, CellLevel, Level},
    weapon::ModeKind,
};

pub(crate) fn ground(x: i32, y: i32) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(0))
}

pub(crate) fn playback_app() -> App {
    let mut app = App::new();
    app.init_resource::<Time>();
    register_playback(&mut app);
    app.insert_resource(ActLog::default());
    app
}

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

pub(crate) fn seed(app: &mut App) {
    let ran = app.world_mut().run_system_once(seed_drawn_state);
    assert!(ran.is_ok(), "the seeding system must run in the fixture");
}

pub(crate) fn append(app: &mut App, actor: Entity, provenance: ActProvenance, deed: ActDeed) {
    let Some(mut log) = app.world_mut().get_resource_mut::<ActLog>() else {
        unreachable!("the fixture inserts an act log");
    };
    log.append(RecordedAct::new(
        actor,
        provenance,
        deed,
        ActWitnesses::unseen(),
    ));
}

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

pub(crate) fn step(app: &mut App, delta: Duration) {
    app.world_mut().resource_mut::<Time>().advance_by(delta);
    let ran = app.world_mut().run_system_once(advance_playback);
    assert!(ran.is_ok(), "the playback cursor must run in the fixture");
}

pub(crate) fn played<M: bevy::ecs::message::Message + Clone>(app: &mut App) -> Vec<Played<M>> {
    app.world_mut()
        .get_resource_mut::<Messages<Played<M>>>()
        .map_or_else(Vec::new, |mut messages| messages.drain().collect())
}

pub(crate) fn shown(app: &App) -> ActSeq {
    app.world().resource::<PlaybackCursor>().shown()
}

pub(crate) fn holding(app: &App) -> bool {
    app.world().resource::<PlaybackCursor>().is_holding()
}

pub(crate) fn tuning(app: &App) -> PlaybackTuning {
    app.world().resource::<PlaybackTuning>().clone()
}

pub(crate) fn drawn_position(app: &App, entity: Entity) -> Option<Position> {
    app.world()
        .get::<DrawnPosition>(entity)
        .map(DrawnPosition::position)
}

pub(crate) fn drawn_pose(app: &App, entity: Entity) -> Option<DrawnPose> {
    app.world().get::<DrawnPose>(entity).copied()
}

pub(crate) const fn drawn_suppressed(pose: DrawnPose) -> bool {
    pose.suppressed()
}
