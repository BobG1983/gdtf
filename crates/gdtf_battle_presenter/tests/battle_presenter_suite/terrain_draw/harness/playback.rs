//! Act logs a case detains, and the manual clock that walks the cursor past them.

use std::time::Duration;

use bevy::{
    app::App,
    ecs::{entity::Entity, message::Messages},
    time::TimeUpdateStrategy,
};
use gdtf_battle_presenter::{PlaybackCursor, PlaybackTuning};
use gdtf_battle_sim::{
    act_log::{ActDeed, ActLog, ActProvenance, ActSeq, ActWitnesses, PositionFacts, RecordedAct},
    entity::TerrainPieceKind,
    ganger::Position,
    occupancy_sync::TerrainPieceDestroyed,
    prelude::CellLevel,
};

// How many manual frames the bounded advance loop is allowed before it gives up.
const MAX_FRAMES: usize = 16;

/// Sequence of the next act-log entry the cursor will show.
pub(crate) fn shown(app: &App) -> ActSeq {
    app.world().resource::<PlaybackCursor>().shown()
}

/// Whether the cursor is dwelling on an entry rather than moving on.
pub(crate) fn holding(app: &App) -> bool {
    app.world().resource::<PlaybackCursor>().is_holding()
}

/// Whether the raw `TerrainPieceDestroyed` buffer is registered in this app.
pub(crate) fn raw_destroyed_present(app: &App) -> bool {
    app.world()
        .contains_resource::<Messages<TerrainPieceDestroyed>>()
}

// One manual frame, longer than the shortest dwell.
pub(crate) fn hold_step(app: &App) -> Duration {
    let minor = *app.world().resource::<PlaybackTuning>().minor_seconds;
    Duration::from_secs_f32(minor.max(0.0) + 0.05)
}

/// An act log holding a detaining minor deed then one smash per entry, whose last
/// sequence it returns.
pub(crate) fn detained_smash_log(
    app: &mut App,
    smashes: &[(CellLevel, TerrainPieceKind)],
) -> ActSeq {
    let actor = app.world_mut().spawn_empty().id();
    let mut log = ActLog::default();
    let mut last = log.append(RecordedAct::new(
        actor,
        ActProvenance::Commanded,
        ActDeed::BleedStarted,
        ActWitnesses::unseen(),
    ));
    for (at, kind) in smashes {
        last = log.append(RecordedAct::new(
            actor,
            ActProvenance::Commanded,
            ActDeed::TerrainPieceSmashed {
                at:   *at,
                kind: *kind,
            },
            ActWitnesses::unseen(),
        ));
    }
    app.world_mut().insert_resource(log);
    last
}

/// An act log holding a detaining minor deed then one move of `actor` to `to`, whose
/// sequence it returns.
pub(crate) fn moved_to_log(app: &mut App, actor: Entity, to: CellLevel) -> ActSeq {
    let mut log = ActLog::default();
    log.append(RecordedAct::new(
        actor,
        ActProvenance::Commanded,
        ActDeed::BleedStarted,
        ActWitnesses::unseen(),
    ));
    let last = log.append(RecordedAct::new(
        actor,
        ActProvenance::Commanded,
        ActDeed::MovedTo {
            position: PositionFacts::new(Position::new(to)),
        },
        ActWitnesses::unseen(),
    ));
    app.world_mut().insert_resource(log);
    last
}

/// Steps the manual clock until the cursor plays `seq`, running `each_frame` after each update.
pub(crate) fn play_past(app: &mut App, seq: ActSeq, each_frame: impl Fn(&App)) {
    let step = hold_step(app);
    app.insert_resource(TimeUpdateStrategy::ManualDuration(step));
    for _ in 0..MAX_FRAMES {
        app.update();
        each_frame(app);
        if shown(app) > seq {
            return;
        }
    }
    assert!(
        shown(app) > seq,
        "the cursor never played the entry {seq:?} within {MAX_FRAMES} manual frames of \
         {step:?} — it is still at {:?}",
        shown(app),
    );
}
