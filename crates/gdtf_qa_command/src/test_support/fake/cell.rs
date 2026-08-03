use bevy::prelude::*;
use gdtf_net_qa_transport::PendingQueue;
use gdtf_qa_protocol::{
    command::{
        CommandAvailability, CommandName, CommandSummary, CommandTiming, RefusalNote,
        UnavailableCode,
    },
    ids::CellNet,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::facts::{FakeFacts, FakeLevel};
use crate::{
    command::QaCommand,
    dispatch::{CommandCall, QaCommandSystems, take_calls},
};

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FakeCellArgs {
        pub cell: CellNet,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct FakeCellReply {
        pub cell:  CellNet,
        pub level: FakeLevel,
}

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
