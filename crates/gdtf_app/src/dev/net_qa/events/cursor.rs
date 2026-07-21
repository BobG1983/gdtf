//! [`QaOutputCursor`] — the QA outbox's OWN read position over the act log (GTW-739).

use bevy::prelude::*;
use gdtf_battle_sim::act_log::ActSeq;

/// How far the QA outbox has drained the sim-owned act log — its [`ActSeq`] read position.
///
/// A process-lifetime [`Resource`] the T6 [`drive_output`](super::drive_output) pump holds,
/// entirely independent of the presenter's playback cursor: both read the ONE
/// non-destructive [`ActLog`](gdtf_battle_sim::act_log::ActLog) ring via
/// [`ActLog::since`](gdtf_battle_sim::act_log::ActLog::since), neither consumes it, and
/// neither can starve the other. It starts at [`ActSeq::START`] and only ever moves forward
/// within a battle; across battles it self-heals (see
/// [`reset_output_cursor_when_idle`](super::reset_output_cursor_when_idle)), so no `gdtf_app`
/// state boundary owns it.
///
/// A named newtype (`no-bare-types.md`): the inner [`ActSeq`] is PRIVATE, read through the
/// derived [`Deref`] / [`position`](Self::position) and moved only through
/// [`advance_to`](Self::advance_to) / [`rewind_to_start`](Self::rewind_to_start).
#[derive(Resource, Deref, Debug, Clone, Copy, Default)]
pub(in crate::dev::net_qa) struct QaOutputCursor(ActSeq);

impl QaOutputCursor {
    /// This cursor's current read position.
    pub(super) const fn position(self) -> ActSeq {
        self.0
    }

    /// Move the cursor forward to `seq` — the sequence one past the last drained entry.
    pub(super) const fn advance_to(&mut self, seq: ActSeq) {
        self.0 = seq;
    }

    /// Whether the cursor is already parked at the start of a battle.
    pub(super) fn is_at_start(self) -> bool {
        self.0 == ActSeq::START
    }

    /// Rewind the cursor to a battle's first sequence — the between-battles reset that keeps
    /// a fresh battle read from the beginning (see
    /// [`reset_output_cursor_when_idle`](super::reset_output_cursor_when_idle)).
    pub(super) const fn rewind_to_start(&mut self) {
        self.0 = ActSeq::START;
    }
}
