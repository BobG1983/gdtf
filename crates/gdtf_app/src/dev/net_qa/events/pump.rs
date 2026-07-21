//! The T6 outbox pump — [`drive_output`] answers a routed `GetOutput` by PROJECTING the
//! sim-owned act log (GTW-739).
//!
//! Registered `.after(SimSystems::Record)` and gated on a live battle by the plugin, it
//! drains the routed [`OutputPayload`] queue and, for each pending drain, reads the ONE
//! non-destructive [`ActLog`] ring from its OWN [`QaOutputCursor`] via
//! [`ActLog::since`](ActLog::since), projects each entry through
//! [`net_event_for`], caps the batch at the client's requested count,
//! advances its cursor, and replies [`Output`](QaResponse::Output).
//!
//! The act log is NEVER drained here: the QA cursor and the presenter's playback cursor are
//! two independent readers over one shared ring, so watching the event stream over the wire
//! never disturbs what the presenter paces on screen. This is why `GetOutput` is not gated
//! on the presenter catching up (GTW-727 C44) — it is the client's observation channel.

use bevy::prelude::*;
use gdtf_battle_sim::{act_log::ActLog, prelude::BattleInProgress};
use gdtf_qa_protocol::{
    envelope::QaResponse,
    events::{DroppedCount, EventBatch, NetEvent},
    ids::EventCap,
};

use super::{cursor::QaOutputCursor, map::net_event_for};
use crate::dev::net_qa::pending::{OutputPayload, PendingQueue};

/// Drain the routed [`OutputPayload`] queue and answer each pending
/// [`GetOutput`](gdtf_qa_protocol::envelope::QaRequest::GetOutput) with a real
/// [`EventBatch`] projected from the act log (GTW-739).
///
/// Idle-cheap: an empty queue returns before touching the log or the cursor. When a drain
/// is pending it answers each request in turn, advancing the cursor so a second drain the
/// same frame sees only what the first left behind (and reports zero dropped). The cursor
/// is reset between battles by [`reset_output_cursor_when_idle`], so it always enters a
/// fresh battle at [`ActSeq::START`](gdtf_battle_sim::act_log::ActSeq::START).
pub(in crate::dev::net_qa) fn drive_output(
    mut queue: ResMut<PendingQueue<OutputPayload>>,
    mut cursor: ResMut<QaOutputCursor>,
    log: Res<ActLog>,
    // Gate-witness only (the plugin's `run_if` guards on it); reading it keeps the system's
    // battle-lifetime contract explicit and matches the T5 snapshot pump's shape.
    _battle: Res<BattleInProgress>,
) {
    let pending = queue.drain_ready();
    if pending.is_empty() {
        return;
    }
    for (payload, responder) in pending {
        let batch = project_batch(&log, &mut cursor, payload.max());
        responder.reply(QaResponse::Output(batch));
    }
}

/// Rewind the QA outbox cursor to a battle's start whenever no act log exists — the
/// between-battles self-heal (GTW-739).
///
/// Mirrors the presenter playback cursor's reset on an absent
/// [`ActLog`](gdtf_battle_sim::act_log::ActLog) (GTW-727 §1.3): a battle's log is inserted
/// and removed with the battle, so an absent log means "no battle in progress" and a new
/// battle restarts sequencing at [`ActSeq::START`](gdtf_battle_sim::act_log::ActSeq::START).
/// Rewinding the cursor there while idle guarantees the next battle is read from the
/// beginning rather than from a position left high by the previous one. Runs ALWAYS (not
/// gated on a live battle), takes `Option<Res<ActLog>>` so it never panics off-battle
/// (`bevy-traps.md` #1), and writes only on the transition to idle so an idle frame never
/// needlessly dirties the resource.
pub(in crate::dev::net_qa) fn reset_output_cursor_when_idle(
    log: Option<Res<ActLog>>,
    mut cursor: ResMut<QaOutputCursor>,
) {
    if log.is_none() && !cursor.is_at_start() {
        cursor.rewind_to_start();
    }
}

/// Project the entries at or after the cursor into a capped [`EventBatch`], advancing the
/// cursor past exactly what was delivered.
///
/// The [`dropped`](EventBatch::dropped) count is the number of act-log entries evicted to
/// overflow BEFORE the cursor reached them — `oldest_seq - cursor` — so a client that fell
/// behind the ring learns its view has a gap. (It counts source entries, an upper bound on
/// missed wire events, since many deeds curate to nothing.) The cap limits WIRE events, not
/// source entries: on hitting it the cursor stops at the next unread entry so the following
/// drain resumes there.
fn project_batch(log: &ActLog, cursor: &mut QaOutputCursor, cap: Option<EventCap>) -> EventBatch {
    let start = cursor.position();
    let dropped = DroppedCount::new(gap_to_u32(log.oldest_seq().distance_from(start)));

    let limit = cap.map(|cap| usize::try_from(*cap).unwrap_or(usize::MAX));
    let mut events: Vec<NetEvent> = Vec::new();
    // Default: the drain consumed the whole retained window, so the cursor lands on `head`.
    let mut next = log.head();
    for entry in log.since(start) {
        if let Some(limit) = limit
            && events.len() >= limit
        {
            // Cap reached — stop BEFORE this entry so the next drain resumes here.
            next = entry.seq();
            break;
        }
        if let Some(event) = net_event_for(entry) {
            events.push(event);
        }
    }
    cursor.advance_to(next);
    EventBatch::new(events, dropped)
}

/// Narrow an act-log sequence gap to the wire [`DroppedCount`]'s `u32`, saturating (a gap
/// wider than `u32::MAX` — impossible in a session — clamps rather than wrapping).
fn gap_to_u32(gap: u64) -> u32 {
    u32::try_from(gap).unwrap_or(u32::MAX)
}
