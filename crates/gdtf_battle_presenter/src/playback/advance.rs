//! [`advance_playback`] — the ONE system that moves the playback cursor and writes the
//! drawn world (GTW-727 C17 / C18 / C19 / C21).

use bevy::prelude::*;
use gdtf_battle_sim::{
    act_log::{ActLog, PositionFacts, VitalsFacts},
    ganger::{Hp, LifeState, Position, Tu, Wounds},
    inflicted_wound::InflictedWounds,
    injuries::InflictedInjuries,
};

use super::{
    apply::{DrawnWriters, show_entry},
    cursor::PlaybackCursor,
    drawn::{DrawnLife, DrawnPosition, DrawnVitals},
    dwell::PlaybackTuning,
    emit::PlayedSignals,
};
use crate::{PendingImpact, ShotProjectile};

/// The live sim state a gap RESYNC reads, as a [`QueryData`] tuple so the system's query
/// list stays under clippy's `type_complexity` gate.
///
/// [`QueryData`]: bevy::ecs::query::QueryData
type ResyncData = (
    Entity,
    &'static Position,
    &'static LifeState,
    &'static Tu,
    &'static Hp,
    &'static Wounds,
    Option<&'static InflictedWounds>,
    Option<&'static InflictedInjuries>,
);

/// The FX-pipeline busy probe — whether any bolt is in flight or any arrival is
/// outstanding.
///
/// Grouped as a [`SystemParam`](bevy::ecs::system::SystemParam) rather than two loose
/// params so the system signature stays readable, and so the "busy" definition has ONE
/// place to live.
#[derive(bevy::ecs::system::SystemParam)]
pub struct FxPipelineProbe<'w, 's> {
    /// Bolts currently in flight.
    projectiles: Query<'w, 's, (), With<ShotProjectile>>,
    /// Arrivals seeded but not yet consumed by the impact animation.
    pending:     Query<'w, 's, (), With<PendingImpact>>,
}

impl FxPipelineProbe<'_, '_> {
    /// Whether the FX pipeline is busy right now.
    fn is_busy(&self) -> bool {
        !self.projectiles.is_empty() || !self.pending.is_empty()
    }
}

/// `Update` ([`Replay`](crate::PresenterSystems::Replay), the FIRST stage of the draw
/// band): advance the playback cursor by AT MOST ONE act-log entry, writing that act's
/// drawn state and emitting its `Played<M>`.
///
/// Running in `Replay` — ahead of `Scene`, `Compose` and `Overlay` in the same chained draw
/// band — means a `Drawn*` component written here is seen as `Changed` by the mirrors that
/// draw from it in the SAME frame, so there is no one-frame lag between showing an act and
/// drawing it.
///
/// ## The loop
///
/// 1. **No log** — no battle is running, so reset the cursor and return. This is the whole
///    of per-battle reset: nothing has to be reset on a state boundary, and no app state
///    owns a presenter resource.
/// 2. **A hold in progress** — tick it. A hold that has not elapsed ends the frame; at most
///    ONE entry is ever released per frame.
/// 3. **A gap** — the cursor has fallen so far behind that entries it never showed have
///    been evicted from the ring. It resyncs the drawn world to live sim state and jumps to
///    the head. See below.
/// 4. **An entry to show** — show it, and start the hold its deed calls for.
///
/// ## What a gap does, and why
///
/// A gap means the presenter is thousands of entries behind, which can only happen if
/// playback has been starved for minutes. Two things are then true: the drawn world is
/// wildly stale, and replaying the retained tail entry-by-entry would take longer than the
/// battle. So the cursor snaps: it resyncs every mirror to the live sim state and jumps
/// straight to the head, counting the skip.
///
/// That is monotone — it only ever moves the drawn world FORWARD, so it can never render a
/// false sequence by rewinding — and it never stalls. It is a deliberate degradation, not a
/// silent one: [`PlaybackCursor::skipped`] counts every entry lost this way.
///
/// Param-only (`bevy-traps.md` #7).
#[expect(
    clippy::too_many_arguments,
    reason = "each param is a distinct, independently-borrowed read the cursor genuinely \
              needs — the clock, the log, the cursor, the dwell table, the FX-busy probe, \
              the gap-resync read, the drawn writers, and the played-signal writers. The \
              two multi-query groups are ALREADY bundled (`FxPipelineProbe` / \
              `DrawnWriters`); bundling the rest would only hide the access set"
)]
pub fn advance_playback(
    time: Res<Time>,
    log: Option<Res<ActLog>>,
    mut cursor: ResMut<PlaybackCursor>,
    tuning: Res<PlaybackTuning>,
    fx: FxPipelineProbe,
    resync: Query<ResyncData>,
    mut drawn: DrawnWriters,
    mut played: PlayedSignals,
) {
    let Some(log) = log else {
        // No battle: reset and stay reset. Self-healing per-battle lifecycle.
        if cursor.shown() != gdtf_battle_sim::act_log::ActSeq::START || cursor.is_holding() {
            cursor.reset();
        }
        return;
    };

    // A log whose head is BEHIND the cursor is a different battle's log (sequence numbers
    // restart at zero per battle), so the cursor rewinds with it.
    if cursor.shown() > log.head() {
        cursor.reset();
    }

    if !cursor.tick_hold(time.delta(), fx.is_busy()) {
        return;
    }

    if cursor.shown() < log.oldest_seq() {
        resync_drawn_world(&resync, &mut drawn);
        cursor.jump_to(log.head());
        return;
    }

    let Some(entry) = log.at(cursor.shown()) else {
        return;
    };
    let hold = show_entry(entry, &tuning, &mut drawn, &mut played);
    cursor.advance_past_shown();
    cursor.hold_for(hold);
}

/// Snap every drawn mirror to the live sim state — the gap recovery.
///
/// Only the mirrors whose value is recoverable from live components are resynced (position,
/// life, vitals). Posture is left alone deliberately: it is cheap to be one act stale and
/// the next posture change corrects it, whereas the magazine lives on weapon entities this
/// query does not reach. Both self-correct on their next recorded transition.
fn resync_drawn_world(resync: &Query<ResyncData>, drawn: &mut DrawnWriters) {
    for (entity, position, life, tu, hp, wounds, inflicted, injuries) in resync {
        if let Ok(mut drawn_position) = drawn.positions.get_mut(entity) {
            drawn_position.set_if_neq(DrawnPosition::new(PositionFacts::new(*position)));
        }
        if let Ok(mut drawn_life) = drawn.lives.get_mut(entity) {
            drawn_life.set_if_neq(DrawnLife::new(*life));
        }
        if let Ok(mut drawn_vitals) = drawn.vitals.get_mut(entity) {
            drawn_vitals.set_if_neq(DrawnVitals::new(VitalsFacts::new(
                *tu,
                *hp,
                *wounds,
                inflicted.cloned().unwrap_or_default(),
                injuries.cloned().unwrap_or_default(),
            )));
        }
    }
}
