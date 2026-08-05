use bevy::prelude::*;
use gdtf_battle_sim::act_log::{ActLog, ActSeq};
use gdtf_net_qa_transport::PendingQueue;
use gdtf_qa_command::{
    command::QaCommand,
    dispatch::{CommandCall, QaCommandSystems, take_calls},
};
use gdtf_qa_protocol::command::{
    CommandAvailability, CommandName, CommandSummary, CommandTiming, RefusalNote, UnavailableCode,
};
use serde::{Deserialize, Serialize};

use super::availability::on_the_battle_screen;
use crate::dev::net_qa::{
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
    /// Oldest sequence to return; absent starts at the oldest line still retained.
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
         exceeds 200. `head` and `oldest` bracket what the ring buffer still holds, so a caller \
         can page and can tell its cursor fell off the end; `dropped` counts the lines before \
         the window. No fog filter, matching the combat log on screen.",
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
        responder.answer(&window(log, &args));
    }
}

/// The lines `args` selects, plus the bounds a caller pages against.
pub(super) fn window(log: &ActLog, args: &LogReadArgs) -> LogReadReply {
    let cap = args
        .cap
        .map_or(DEFAULT_CAP, |asked| LogReadCap::new((*asked).min(*MAX_CAP)));
    let cursor = args.since.map_or_else(
        || log.oldest_seq(),
        |since| ActSeq::new(*since).max(log.oldest_seq()),
    );
    let selected = log.since(cursor).count();
    let keep = usize::try_from(*cap).unwrap_or(usize::MAX);
    let skipped = selected.saturating_sub(keep);
    let entries: Vec<LogEntryNet> = log
        .since(cursor)
        .skip(skipped)
        .map(LogEntryNet::from_sim)
        .collect();
    let before_window = u32::try_from(skipped).unwrap_or(u32::MAX);
    LogReadReply {
        entries,
        cap,
        head: ActSeqNet::new(*log.head()),
        oldest: ActSeqNet::new(*log.oldest_seq()),
        dropped: LogDroppedCount::new((*log.dropped()).saturating_add(before_window)),
    }
}
