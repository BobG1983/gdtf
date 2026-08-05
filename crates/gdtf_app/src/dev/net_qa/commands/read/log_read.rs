use bevy::prelude::*;
use gdtf_battle_sim::act_log::ActLog;
use gdtf_net_qa_transport::PendingQueue;
use gdtf_qa_command::{
    command::QaCommand,
    dispatch::{CommandCall, QaCommandSystems, take_calls},
};
use gdtf_qa_protocol::command::{CommandAvailability, CommandName, CommandSummary, CommandTiming};
use serde::{Deserialize, Serialize};

use super::availability::on_the_battle_screen;
use crate::dev::net_qa::{
    facts::GameFacts,
    wire::log::{LogDroppedCount, LogEntryNet, LogReadCap},
};

/// Lines returned when the caller names no cap.
const DEFAULT_CAP: LogReadCap = LogReadCap::new(64);

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct LogReadArgs {
    /// How many of the newest lines to return; the caller picks it, absent means 64.
    #[serde(default)]
    cap: Option<LogReadCap>,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct LogReadReply {
    entries: Vec<LogEntryNet>,
    cap:     LogReadCap,
    dropped: LogDroppedCount,
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
         newest window first capped by `cap`. `dropped` counts the lines before that window, \
         including any the ring buffer has already thrown away. No fog filter, matching the \
         combat log on screen.",
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
        let cap = args.cap.unwrap_or(DEFAULT_CAP);
        let (entries, dropped) = window(log.as_deref(), cap);
        responder.answer(&LogReadReply {
            entries,
            cap,
            dropped,
        });
    }
}

fn window(log: Option<&ActLog>, cap: LogReadCap) -> (Vec<LogEntryNet>, LogDroppedCount) {
    let Some(log) = log else {
        return (Vec::new(), LogDroppedCount::new(0));
    };
    let keep = usize::try_from(*cap).unwrap_or(usize::MAX);
    let skipped = log.len().saturating_sub(keep);
    let entries: Vec<LogEntryNet> = log
        .since(log.oldest_seq())
        .skip(skipped)
        .map(LogEntryNet::from_sim)
        .collect();
    let before_window = u32::try_from(skipped).unwrap_or(u32::MAX);
    let dropped = LogDroppedCount::new((*log.dropped()).saturating_add(before_window));
    (entries, dropped)
}
