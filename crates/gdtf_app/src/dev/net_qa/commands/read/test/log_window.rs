//! Which lines the log window returns, and the bounds it puts on a caller's cap.

use bevy::prelude::Entity;
use gdtf_battle_sim::{
    act_log::{ActDeed, ActLog, ActProvenance, ActWitnesses, RecordedAct, WatchingFactions},
    prelude::Faction,
};

use crate::dev::net_qa::commands::read::log_read::{LogReadArgs, window};

/// More lines than any cap this command applies, so a ceiling has something to cut.
const WRITTEN: u64 = 512;

/// Sequence a paging caller resumes from, far enough in to leave lines on both sides.
const CURSOR: u64 = 500;

/// The gang doing the asking in every case here.
pub(super) const PLAYER: Faction = Faction::new(0);

/// The gang standing where the asking gang cannot see.
pub(super) const ENEMY: Faction = Faction::new(1);

/// Witnesses saying `gang` watched the act and could name whoever did it.
pub(super) fn watched_by(gang: Faction) -> ActWitnesses {
    let watching = WatchingFactions::new([gang]);
    ActWitnesses::new(watching.clone(), watching)
}

/// Witnesses saying `gang` watched the act but could not name whoever did it.
pub(super) fn watched_unnamed(gang: Faction) -> ActWitnesses {
    ActWitnesses::new(WatchingFactions::new([gang]), WatchingFactions::nobody())
}

/// A log holding [`WRITTEN`] lines, sequenced from the start and inside the ring buffer.
fn a_flooded_log() -> ActLog {
    let mut log = ActLog::default();
    for _ in 0..WRITTEN {
        log.append(RecordedAct::new(
            Entity::PLACEHOLDER,
            ActProvenance::Clock,
            ActDeed::TurnBegan { now_active: PLAYER },
            watched_by(PLAYER),
        ));
    }
    log
}

/// Parse arguments the way the wire hands them over, so a case uses the caller's own text.
pub(super) fn args(text: &str) -> LogReadArgs {
    match ron::de::from_str::<LogReadArgs>(text) {
        Ok(parsed) => parsed,
        Err(fault) => {
            unreachable!("`{text}` is the argument shape this command publishes: {fault}")
        }
    }
}

#[test]
fn a_read_that_names_no_cap_stops_well_short_of_the_whole_log() {
    let log = a_flooded_log();
    let reply = window(&log, &args("()"), Some(PLAYER));

    assert!(
        u64::try_from(reply.entries.len()).unwrap_or(u64::MAX) < WRITTEN,
        "a read naming no cap must not hand back a {WRITTEN}-line log: {reply:?}",
    );
    assert_eq!(
        reply.entries.len(),
        usize::try_from(*reply.cap).unwrap_or(usize::MAX),
        "the default window returns exactly the cap it reports: {reply:?}",
    );
}

#[test]
fn a_cap_past_the_ceiling_comes_back_as_the_ceiling() {
    let log = a_flooded_log();
    let greedy = window(&log, &args("(cap:Some(100000))"), Some(PLAYER));
    let modest = window(&log, &args("(cap:Some(3))"), Some(PLAYER));

    assert!(
        u64::from(*greedy.cap) < WRITTEN,
        "the cap a caller names is a request, not a licence to dump the log: {greedy:?}",
    );
    assert_eq!(
        greedy.entries.len(),
        usize::try_from(*greedy.cap).unwrap_or(usize::MAX),
        "the clamped cap is the one the window applied, not just the one it reported: {greedy:?}",
    );
    assert_eq!(
        (modest.entries.len(), *modest.cap),
        (3, 3),
        "a cap under the ceiling is honoured as asked: {modest:?}",
    );
}

#[test]
fn a_window_keeps_the_newest_lines_and_counts_what_it_left_out() {
    let log = a_flooded_log();
    let reply = window(&log, &args("(cap:Some(4))"), Some(PLAYER));

    let Some(newest) = reply.entries.last() else {
        unreachable!("a four-line window over a full log is not empty");
    };
    assert_eq!(
        *newest.seq,
        WRITTEN.saturating_sub(1),
        "the window ends on the newest line the log holds: {reply:?}",
    );
    assert_eq!(
        u64::from(*reply.dropped),
        WRITTEN.saturating_sub(4),
        "every line the cap left outside the window is counted: {reply:?}",
    );
}

#[test]
fn a_cursor_starts_the_window_at_the_sequence_the_caller_named() {
    let log = a_flooded_log();
    let reply = window(
        &log,
        &args(&format!("(since:Some({CURSOR}),cap:Some(200))")),
        Some(PLAYER),
    );

    let Some(first) = reply.entries.first() else {
        unreachable!("sequence {CURSOR} is still retained in a {WRITTEN}-line log");
    };
    assert_eq!(
        *first.seq, CURSOR,
        "the page begins where the caller pointed it: {reply:?}",
    );
    assert_eq!(
        u64::try_from(reply.entries.len()).unwrap_or(u64::MAX),
        WRITTEN.saturating_sub(CURSOR),
        "the page runs from the cursor to the newest line and stops: {reply:?}",
    );
}

#[test]
fn a_read_brackets_the_window_with_what_the_buffer_still_holds() {
    let log = a_flooded_log();
    let reply = window(&log, &args("(cap:Some(4))"), Some(PLAYER));

    assert_eq!(
        *reply.head, WRITTEN,
        "head names the sequence the log would assign next, so a caller pages from it: \
         {reply:?}",
    );
    assert_eq!(
        *reply.oldest, 0,
        "nothing has fallen out of a buffer this size, so the oldest retained line is the \
         first one written: {reply:?}",
    );
}
