//! [`FakeCell`] — a fake command with a STRUCTURED argument type, available only once the
//! fake model is loaded.

use bevy::prelude::*;
use gdtf_net_qa_transport::PendingQueue;
use gdtf_qa_protocol::{
    command::{CommandAvailability, CommandName, CommandSummary, RefusalNote, UnavailableCode},
    ids::CellNet,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::facts::{FakeFacts, FakeLevel};
use crate::{
    command::QaCommand,
    dispatch::{CommandCall, QaCommandSystems, take_calls},
};

/// [`FakeCell`]'s arguments — one [`CellNet`], a structured (non-primitive) argument.
///
/// The argument type embeds a `gdtf_qa_protocol` WIRE ID rather than a look-alike of its
/// own, and still derives its whole schema. That is the point of this crate depending on
/// `gdtf_qa_protocol` with its `schema` feature on (GTW-941's dependency line): a host's
/// argument type names the same coordinate the wire already defines, and `schemars` walks
/// into it. Take the feature away and this struct stops compiling.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FakeCellArgs {
    /// The cell to read.
    pub cell: CellNet,
}

/// [`FakeCell`]'s reply — the cell it was asked about and the level it was read at.
#[derive(Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct FakeCellReply {
    /// The cell that was read.
    pub cell:  CellNet,
    /// The level it was read at.
    pub level: FakeLevel,
}

/// A fake command that needs the fake model loaded.
///
/// The state-dependent half of the availability test: unavailable with
/// [`MissingModel`](UnavailableCode::MissingModel) while nothing is loaded, available once
/// it is — and the catalogue publishes exactly that, from this same predicate.
pub struct FakeCell;

impl QaCommand for FakeCell {
    type Args = FakeCellArgs;
    type Facts = FakeFacts;
    type Reply = FakeCellReply;

    const NAME: CommandName = CommandName::from_static("fake.cell");
    const SUMMARY: CommandSummary =
        CommandSummary::from_static("Read one cell. Needs the fake model loaded.");

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

/// Answer every claimed [`FakeCell`] call with the cell it asked about.
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
