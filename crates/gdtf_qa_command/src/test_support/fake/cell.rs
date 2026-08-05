//! Fake command that echoes a cell and the host level.

use bevy::prelude::*;
use gdtf_net_qa_transport::PendingQueue;
use gdtf_qa_protocol::{
    command::{
        CommandAvailability, CommandName, CommandSummary, CommandTiming, RefusalNote,
        UnavailableCode,
    },
    ids::CellNet,
};
use serde::{Deserialize, Serialize};

use super::facts::{FakeFacts, FakeLevel};
use crate::{
    command::QaCommand,
    dispatch::{CommandCall, QaCommandSystems, take_calls},
};

/// Arguments for `fake.cell`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FakeCellArgs {
    /// Cell to read.
    pub cell: CellNet,
}

/// Reply for `fake.cell`.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct FakeCellReply {
    /// Echoed cell.
    pub cell:  CellNet,
    /// Host level at the time of the call.
    pub level: FakeLevel,
}

/// Read one cell; requires the fake model to be loaded.
pub struct FakeCell;

impl QaCommand for FakeCell {
    type Args = FakeCellArgs;
    type Facts = FakeFacts;
    type Reply = FakeCellReply;

    const NAME: CommandName = CommandName::from_static("fake.cell");
    const SUMMARY: CommandSummary =
        CommandSummary::from_static("Read one cell. Needs the fake model loaded.");

    const TIMING: CommandTiming = CommandTiming::Immediate;

    fn availability(facts: &FakeFacts) -> CommandAvailability {
        if *facts.ready() {
            return CommandAvailability::Available;
        }
        CommandAvailability::Unavailable {
            code: UnavailableCode::MissingModel,
            note: RefusalNote::from_static("the fake host has loaded no cell model"),
        }
    }

    fn register_handler(app: &mut App) {
        app.add_systems(Update, handle_fake_cell.after(QaCommandSystems::Claim));
    }
}

fn handle_fake_cell(facts: Res<FakeFacts>, mut queue: ResMut<PendingQueue<CommandCall<FakeCell>>>) {
    if queue.is_empty() {
        return;
    }
    for (args, responder) in take_calls::<FakeCell>(&mut queue) {
        responder.answer(&FakeCellReply {
            cell:  args.cell,
            level: facts.level(),
        });
    }
}
