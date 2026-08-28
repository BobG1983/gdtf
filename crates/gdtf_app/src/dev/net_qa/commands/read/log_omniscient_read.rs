use bevy::prelude::*;
use gdtf_battle_sim::act_log::ActLog;
use gdtf_net_qa_transport::PendingQueue;
use gdtf_qa_command::{
    command::QaCommand,
    dispatch::{CommandCall, QaCommandSystems, take_calls},
};
use gdtf_qa_protocol::command::{
    CommandAvailability, CommandName, CommandSummary, CommandTiming, RefusalNote, UnavailableCode,
};
use serde::Deserialize;

use super::{
    availability::on_the_battle_screen,
    log_read::{LogReadReply, capped_at, cursor_from},
};
use crate::dev::net_qa::{
    facts::GameFacts,
    wire::{
        act::ActSeqNet,
        log::{LogDroppedCount, LogEntryNet, LogReadCap},
    },
};

/// Why an unfiltered read answers nothing while the battlescape is still building its battle.
const NO_ACT_LOG: RefusalNote =
    RefusalNote::from_static("the battle runtime that owns the act log is not loaded yet");

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct LogOmniscientReadArgs {
    /// Oldest sequence to return; absent starts at the oldest line the log holds.
    #[serde(default)]
    since: Option<ActSeqNet>,
    /// How many of the newest lines to return; absent means 50, and 200 is the ceiling.
    #[serde(default)]
    cap:   Option<LogReadCap>,
}

pub(crate) struct LogOmniscientRead;

impl QaCommand for LogOmniscientRead {
    type Args = LogOmniscientReadArgs;
    type Facts = GameFacts;
    type Parked = ();
    type Reply = LogReadReply;

    const NAME: CommandName = CommandName::from_static("log.omniscient_read");
    const SUMMARY: CommandSummary = CommandSummary::from_static(
        "Read the tail of the act log with no fog filter — every act by every faction, \
         including acts the squad could not observe. Using this is cheating: it is for \
         testing only, and it must never back a claim about what a player can see. Use \
         `log.read` for that.",
    );
    const TIMING: CommandTiming = CommandTiming::Immediate;

    fn availability(facts: &GameFacts) -> CommandAvailability {
        on_the_battle_screen(*facts)
    }

    fn register_handler(app: &mut App) {
        app.add_systems(
            Update,
            handle_log_omniscient_read.after(QaCommandSystems::Claim),
        );
    }
}

fn handle_log_omniscient_read(
    log: Option<Res<ActLog>>,
    mut queue: ResMut<PendingQueue<CommandCall<LogOmniscientRead>>>,
) {
    if queue.is_empty() {
        return;
    }
    for (args, responder) in take_calls::<LogOmniscientRead>(&mut queue) {
        let Some(log) = log.as_deref() else {
            responder.unavailable(UnavailableCode::MissingModel, NO_ACT_LOG);
            continue;
        };
        responder.answer(&whole_window(log, &args));
    }
}

/// Every line `args` selects, fully identified.
pub(super) fn whole_window(log: &ActLog, args: &LogOmniscientReadArgs) -> LogReadReply {
    let cap = capped_at(args.cap);
    let cursor = cursor_from(log, args.since);
    let selected = log.since(cursor).count();
    let keep = usize::try_from(*cap).unwrap_or(usize::MAX);
    let skipped = selected.saturating_sub(keep);
    let entries: Vec<LogEntryNet> = log
        .since(cursor)
        .skip(skipped)
        .map(LogEntryNet::omniscient)
        .collect();
    let before_window = LogDroppedCount::new(u32::try_from(skipped).unwrap_or(u32::MAX));
    LogReadReply {
        entries,
        cap,
        head: ActSeqNet::new(*log.head()),
        oldest: ActSeqNet::new(*log.oldest_seq()),
        dropped: before_window,
    }
}
