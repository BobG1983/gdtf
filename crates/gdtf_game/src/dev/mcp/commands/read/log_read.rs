use bevy::prelude::*;
use cobalt_mcp_command::{
    command::QaCommand,
    dispatch::{CommandCall, QaCommandSystems, take_calls},
};
use cobalt_mcp_protocol::command::{
    CommandAvailability, CommandName, CommandSummary, CommandTiming, RefusalNote, UnavailableCode,
};
use cobalt_mcp_transport::PendingQueue;
use gdtf_battle_sim::{
    act_log::{ActLog, ActSeq},
    battle::PlayerFaction,
    ganger::Faction,
    visibility::classify_act,
};
use serde::{Deserialize, Serialize};

use super::availability::on_the_battle_screen;
use crate::dev::mcp::{
    facts::GameFacts,
    wire::{
        act::ActSeqNet,
        log::{LogDroppedCount, LogEntryNet, LogReadCap},
    },
};

/// Lines returned when the caller names no cap.
const DEFAULT_CAP: LogReadCap = LogReadCap::new(50);

/// Most lines one read will ever return, however large a cap the caller names.
const MAX_CAP: LogReadCap = LogReadCap::new(200);

/// Why a read answers nothing while the battlescape is still building its battle.
const NO_ACT_LOG: RefusalNote =
    RefusalNote::from_static("the battle runtime that owns the act log is not loaded yet");

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct LogReadArgs {
    /// Oldest sequence to return; absent starts at the oldest line the log holds.
    #[serde(default)]
    since: Option<ActSeqNet>,
    /// How many of the newest lines to return; absent means 50, and 200 is the ceiling.
    #[serde(default)]
    cap:   Option<LogReadCap>,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct LogReadReply {
    pub(super) entries: Vec<LogEntryNet>,
    pub(super) cap:     LogReadCap,
    pub(super) head:    ActSeqNet,
    pub(super) oldest:  ActSeqNet,
    pub(super) dropped: LogDroppedCount,
}

pub(crate) struct LogRead;

impl QaCommand for LogRead {
    type Args = LogReadArgs;
    type Facts = GameFacts;
    type Parked = ();
    type Reply = LogReadReply;

    const NAME: CommandName = CommandName::from_static("log.read");
    const SUMMARY: CommandSummary = CommandSummary::from_static(
        "Read the tail of the act log — sequence, actor, provenance and deed kind per line, \
         starting at `since` and keeping the newest `cap`, which defaults to 50 and never \
         exceeds 200. The log keeps every line a battle writes, so `head` and `oldest` bracket \
         all of it and a caller pages from `head`; `dropped` counts the lines the cap left \
         before the window. Fog-gated the way the combat log on screen is: a line the player's \
         gang could not observe is dropped, and an actor it could not identify comes back with no \
         token, so a reply may hold fewer lines than `cap`.",
    );
    const TIMING: CommandTiming = CommandTiming::Immediate;

    fn availability(facts: &GameFacts) -> CommandAvailability {
        on_the_battle_screen(*facts)
    }

    fn register_handler(app: &mut App) {
        app.add_systems(Update, handle_log_read.after(QaCommandSystems::Claim));
    }
}

fn handle_log_read(
    log: Option<Res<ActLog>>,
    player: Option<Res<PlayerFaction>>,
    mut queue: ResMut<PendingQueue<CommandCall<LogRead>>>,
) {
    if queue.is_empty() {
        return;
    }
    for (args, responder) in take_calls::<LogRead>(&mut queue) {
        let Some(log) = log.as_deref() else {
            responder.unavailable(UnavailableCode::MissingModel, NO_ACT_LOG);
            continue;
        };
        let asking = player.as_deref().map(|player| **player);
        responder.answer(&window(log, &args, asking));
    }
}

/// The cap a read applies: the caller's, clamped to the ceiling, or the default.
pub(super) fn capped_at(asked: Option<LogReadCap>) -> LogReadCap {
    asked.map_or(DEFAULT_CAP, |asked| LogReadCap::new((*asked).min(*MAX_CAP)))
}

/// The sequence a read starts at, never older than the log's own oldest line.
pub(super) fn cursor_from(log: &ActLog, since: Option<ActSeqNet>) -> ActSeq {
    since.map_or_else(
        || log.oldest_seq(),
        |since| ActSeq::new(*since).max(log.oldest_seq()),
    )
}

/// The lines `args` selects that `asking` could observe, plus the bounds a caller pages against.
/// The cap is applied first, so a filtered reply may hold fewer lines than the cap.
pub(super) fn window(log: &ActLog, args: &LogReadArgs, asking: Option<Faction>) -> LogReadReply {
    let cap = capped_at(args.cap);
    let cursor = cursor_from(log, args.since);
    let selected = log.since(cursor).count();
    let keep = usize::try_from(*cap).unwrap_or(usize::MAX);
    let skipped = selected.saturating_sub(keep);
    let entries: Vec<LogEntryNet> = match asking {
        Some(asking) => log
            .since(cursor)
            .skip(skipped)
            .filter_map(|entry| {
                let seen = entry.witnesses();
                let view = classify_act(seen.observed_by(asking), seen.identifies_actor(asking));
                LogEntryNet::from_sim(entry, asking, view)
            })
            .collect(),
        None => Vec::new(),
    };
    let before_window = LogDroppedCount::new(u32::try_from(skipped).unwrap_or(u32::MAX));
    LogReadReply {
        entries,
        cap,
        head: ActSeqNet::new(*log.head()),
        oldest: ActSeqNet::new(*log.oldest_seq()),
        dropped: before_window,
    }
}
