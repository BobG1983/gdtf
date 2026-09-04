use cobalt_mcp_protocol::{command::RunOptions, message::QaResponse};
use gdtf_game::qa_wire::{
    act::ActSeqNet,
    deed::ActDeedKindNet,
    log::{ActProvenanceNet, LogDroppedCount, LogEntryNet, LogReadCap},
    token::GangerToken,
};
use serde::Deserialize;

use super::{
    battle_setup::{
        FLOODED_LOG_LINES, LOG_LINES_WRITTEN, LoggedActor, battle_with_a_flooded_log,
        battle_with_log_lines,
    },
    command_exchange::{
        LOG_READ, assert_refused_off_the_battle_screen, exchange_expected, ran_body, run,
    },
    socket_support::{TestError, TestResult, game_app_listening},
};

/// Cap the tight read applies, small enough to leave written lines outside the window.
const TIGHT_CAP: u32 = 2;

/// A cap far past any ceiling the command could sanely apply.
const GREEDY_CAP: u32 = 100_000;

#[derive(Debug, Deserialize)]
struct LogBody {
    entries: Vec<LogEntryNet>,
    cap:     LogReadCap,
    head:    ActSeqNet,
    oldest:  ActSeqNet,
    dropped: LogDroppedCount,
}

fn decode(reply: Option<QaResponse>) -> Result<LogBody, TestError> {
    let Some(reply) = reply else {
        return Err("log.read produced no reply".into());
    };
    let body = ran_body(LOG_READ, reply)?;
    ron::de::from_str::<LogBody>(&body)
        .map_err(|fault| format!("the log body must decode: {fault} — {body}").into())
}

/// Read the whole log and a tightly capped window of it, from one battle with written lines.
fn default_and_capped() -> Result<(LogBody, LogBody, LoggedActor), TestError> {
    let (replies, logged) = exchange_expected(battle_with_log_lines, |_logged| {
        vec![
            run(LOG_READ, "()", RunOptions::default()),
            run(
                LOG_READ,
                &format!("(cap:Some({TIGHT_CAP}))"),
                RunOptions::default(),
            ),
        ]
    })?;
    let mut replies = replies.into_iter();
    let uncapped = decode(replies.next())?;
    let capped = decode(replies.next())?;
    Ok((uncapped, capped, logged))
}

#[test]
fn the_lines_the_battle_logged_come_back_as_the_sim_recorded_them() -> TestResult {
    let (uncapped, _capped, logged) = default_and_capped()?;

    let actor = GangerToken::new(logged.actor.to_bits());
    let mine: Vec<&LogEntryNet> = uncapped
        .entries
        .iter()
        .filter(|entry| entry.actor == Some(actor) && entry.kind == ActDeedKindNet::TurnBegan)
        .collect();
    assert!(
        u32::try_from(mine.len()).unwrap_or(u32::MAX) >= LOG_LINES_WRITTEN,
        "every line the battle logged for this ganger comes back, mirrored deed and all — the \
         player's own gang observed each of them, so the fog filter keeps them: {uncapped:?}",
    );
    for entry in mine {
        assert_eq!(
            entry.provenance,
            ActProvenanceNet::Commanded,
            "the line carries the provenance it was recorded with: {entry:?}",
        );
    }
    Ok(())
}

#[test]
fn a_tight_cap_returns_exactly_that_many_of_the_newest_lines() -> TestResult {
    let (mut uncapped, capped, _logged) = default_and_capped()?;

    assert_eq!(
        u32::try_from(capped.entries.len()).unwrap_or(u32::MAX),
        TIGHT_CAP,
        "the log held more lines than the cap, so the cap decides the window size: {capped:?} \
         against {uncapped:?}",
    );
    assert_eq!(
        capped.cap,
        LogReadCap::new(TIGHT_CAP),
        "the reply reports the cap it actually applied: {capped:?}",
    );
    let newest = uncapped.entries.len().saturating_sub(capped.entries.len());
    let tail = uncapped.entries.split_off(newest);
    assert_eq!(
        capped.entries, tail,
        "the window keeps the newest lines and skips the older ones: {capped:?} against \
         {tail:?}",
    );
    Ok(())
}

#[test]
fn a_tighter_cap_counts_every_line_it_left_outside_the_window() -> TestResult {
    let (uncapped, capped, _logged) = default_and_capped()?;

    let skipped = uncapped.entries.len().saturating_sub(capped.entries.len());
    let skipped = u32::try_from(skipped).unwrap_or(u32::MAX);
    assert!(
        skipped > 0,
        "the battle logged more lines than the tight cap keeps, so some were skipped: \
         {uncapped:?}",
    );
    assert_eq!(
        *capped.dropped,
        (*uncapped.dropped).saturating_add(skipped),
        "cutting the window reports each line it left outside on top of what the wider read \
         had already left out: {capped:?} against {uncapped:?}",
    );
    let sequences: Vec<u64> = uncapped.entries.iter().map(|entry| *entry.seq).collect();
    assert!(
        sequences.is_sorted(),
        "the act log hands back its lines in sequence order: {sequences:?}",
    );
    Ok(())
}

#[test]
fn a_read_that_names_no_cap_stops_at_the_default_window() -> TestResult {
    let (replies, _logged) = exchange_expected(battle_with_a_flooded_log, |_logged| {
        vec![run(LOG_READ, "()", RunOptions::default())]
    })?;
    let uncapped = decode(replies.into_iter().next())?;
    let kept = u32::try_from(uncapped.entries.len()).unwrap_or(u32::MAX);
    let cap = *uncapped.cap;

    assert!(
        cap < FLOODED_LOG_LINES,
        "the fixture writes {FLOODED_LOG_LINES} lines, more than the default window keeps, so \
         the window has something to leave out — write more if the default grew past it: \
         cap {cap}, kept {kept}",
    );
    assert_eq!(
        kept,
        cap,
        "a read that names no cap still stops at the default window rather than handing back \
         the whole log: kept {kept} of {FLOODED_LOG_LINES} written, dropped {dropped}",
        dropped = *uncapped.dropped,
    );
    Ok(())
}

#[test]
fn a_cap_past_the_ceiling_is_clamped_rather_than_handing_back_the_whole_log() -> TestResult {
    let (replies, _logged) = exchange_expected(battle_with_a_flooded_log, |_logged| {
        vec![run(
            LOG_READ,
            &format!("(cap:Some({GREEDY_CAP}))"),
            RunOptions::default(),
        )]
    })?;
    let greedy = decode(replies.into_iter().next())?;
    let applied = *greedy.cap;
    let kept = u32::try_from(greedy.entries.len()).unwrap_or(u32::MAX);

    assert!(
        applied < GREEDY_CAP,
        "a caller asking for {GREEDY_CAP} lines gets the ceiling instead, or the read is the \
         whole-log dump the QA surface exists to avoid: applied {applied}",
    );
    assert!(
        applied < FLOODED_LOG_LINES,
        "the fixture writes {FLOODED_LOG_LINES} lines, so the ceiling must sit under that for \
         this case to prove anything — raise the fixture if the ceiling grew: applied {applied}",
    );
    assert_eq!(
        kept, applied,
        "the window the reply reports is the window it returned: kept {kept} of \
         {FLOODED_LOG_LINES} written",
    );
    Ok(())
}

#[test]
fn a_read_reports_the_bounds_of_what_the_ring_buffer_still_holds() -> TestResult {
    let (uncapped, capped, _logged) = default_and_capped()?;

    for body in [&uncapped, &capped] {
        assert!(
            body.oldest <= body.head,
            "the oldest retained line never sits past the next sequence the log would \
             assign: {body:?}",
        );
    }
    let Some(newest) = uncapped.entries.last() else {
        return Err("the fixture wrote lines, so the default window returns some".into());
    };
    assert!(
        newest.seq < uncapped.head,
        "head names the sequence the log would assign next, so every returned line sits below \
         it: {newest:?} against {uncapped:?}",
    );
    assert_eq!(
        capped.head, uncapped.head,
        "capping the window changes which lines come back, never where the log has got to: \
         {capped:?} against {uncapped:?}",
    );
    Ok(())
}

#[test]
fn the_log_read_refuses_off_the_battle_screen() -> TestResult {
    assert_refused_off_the_battle_screen(game_app_listening, LOG_READ, "()")?;
    Ok(())
}
