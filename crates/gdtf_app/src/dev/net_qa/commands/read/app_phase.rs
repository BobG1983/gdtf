//! `app.phase` — read where the app is, at every level of its state machine (GTW-942).
//!
//! The GAME host's first command, and the worked example
//! `docs/tooling/qa-commands.md` is written from. It was chosen first because of what it
//! exercises rather than what it does: it needs no battle, so it is callable the instant
//! the process boots; its [`AppPhaseArgs`] is the EMPTY object, which drives the decode
//! path at its edge; its [`AppPhaseReply`] is a nested record, so schema derivation is
//! exercised properly rather than on a flat pair of scalars; and it closes the gap where
//! the four sub-states were unreadable over the wire — which is what makes it the command
//! every later integration test waits for a state with.

use bevy::prelude::*;
use gdtf_net_qa_transport::PendingQueue;
use gdtf_qa_command::{
    command::QaCommand,
    dispatch::{CommandCall, QaCommandSystems, take_calls},
};
use gdtf_qa_protocol::command::{CommandAvailability, CommandName, CommandSummary, CommandTiming};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::dev::net_qa::{
    facts::{GameFacts, GameFactsParam},
    wire::AppPhaseNet,
};

/// `app.phase`'s arguments: none at all.
///
/// `deny_unknown_fields` is what turns "you sent a field I do not have" into a
/// `BadArguments` answer carrying this type's own derived schema, instead of a silently
/// ignored key — and it is what puts `"additionalProperties": false` in the published
/// schema, so a client can see the strictness before it calls.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct AppPhaseArgs {}

/// `app.phase`'s reply: the whole five-level state tuple.
///
/// A record rather than a bare [`AppPhaseNet`] so a later fact can join it without moving
/// what a client already reads.
#[derive(Debug, Serialize, JsonSchema)]
pub(crate) struct AppPhaseReply {
    /// Where the app is, at every level of its state machine.
    phase: AppPhaseNet,
}

/// Read where the app is, at every level of its state machine.
pub(crate) struct AppPhase;

impl QaCommand for AppPhase {
    type Args = AppPhaseArgs;
    type Facts = GameFacts;
    type Reply = AppPhaseReply;

    const NAME: CommandName = CommandName::from_static("app.phase");
    const SUMMARY: CommandSummary = CommandSummary::from_static(
        "Read where the app is at every level of its state machine — the lifecycle phase \
         plus the running screen, game layer, battle phase and aftermath phase where each \
         is live. Needs no battle, so it answers from the moment the process boots; poll \
         it to wait for a state.",
    );
    const TIMING: CommandTiming = CommandTiming::Immediate;

    /// Always available.
    ///
    /// Deliberately not a function of anything: a command that reports WHERE the app is
    /// must be answerable wherever the app is, or a client cannot find out that it is
    /// somewhere the command refuses. That is also what makes it the poll every other
    /// command's precondition is discovered through.
    fn availability(_facts: &GameFacts) -> CommandAvailability {
        CommandAvailability::Available
    }

    fn register_handler(app: &mut App) {
        app.add_systems(Update, handle_app_phase.after(QaCommandSystems::Claim));
    }
}

/// Answer every claimed `app.phase` call from the frame's facts.
///
/// `.after(QaCommandSystems::Claim)` (declared in
/// [`register_handler`](AppPhase::register_handler)) is the whole ordering requirement:
/// the decode step fills this queue, so a handler that ran before it would answer every
/// call a frame late. Nothing else about the schedule matters — the answer is a read of
/// state resources, not of the sim.
///
/// The `is_empty` early-out takes `&self` through the `ResMut`, so an idle frame — which
/// is very nearly every frame — never dirties the queue's change-detection flag.
fn handle_app_phase(facts: GameFactsParam, mut queue: ResMut<PendingQueue<CommandCall<AppPhase>>>) {
    if queue.is_empty() {
        return;
    }
    let sampled = facts.sample();
    for (_args, responder) in take_calls::<AppPhase>(&mut queue) {
        responder.answer(&AppPhaseReply {
            phase: sampled.phase(),
        });
    }
}
