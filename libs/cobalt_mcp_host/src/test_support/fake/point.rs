//! Fake command that echoes a point and the host level.

use bevy::prelude::*;
use cobalt_mcp_protocol::command::{
    CommandAvailability, CommandName, CommandSummary, CommandTiming, RefusalNote, UnavailableCode,
};
use serde::{Deserialize, Serialize};

use super::facts::{FakeFacts, FakeLevel};
use crate::{
    command::McpCommand,
    dispatch::{CommandCall, McpCommandSystems, take_calls},
    transport::PendingQueue,
};

/// First axis of a fake point.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct FakePointX(i32);

impl FakePointX {
    /// Wrap a first-axis value.
    #[must_use]
    pub const fn new(x: i32) -> Self {
        Self(x)
    }
}

/// Second axis of a fake point.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct FakePointY(i32);

impl FakePointY {
    /// Wrap a second-axis value.
    #[must_use]
    pub const fn new(y: i32) -> Self {
        Self(y)
    }
}

/// A pair of axes, the nested record the fake command carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FakePointPair {
    /// First axis.
    pub x: FakePointX,
    /// Second axis.
    pub y: FakePointY,
}

impl FakePointPair {
    /// Build a pair from its two axes.
    #[must_use]
    pub const fn new(x: FakePointX, y: FakePointY) -> Self {
        Self { x, y }
    }
}

/// Arguments for `fake.point`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FakePointArgs {
    /// Point to read.
    pub point: FakePointPair,
}

/// Reply for `fake.point`.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct FakePointReply {
    /// Echoed point.
    pub point: FakePointPair,
    /// Host level at the time of the call.
    pub level: FakeLevel,
}

/// Read one point; requires the fake model to be loaded.
pub struct FakePoint;

impl McpCommand for FakePoint {
    type Args = FakePointArgs;
    type Facts = FakeFacts;
    type Parked = ();
    type Reply = FakePointReply;

    const NAME: CommandName = CommandName::from_static("fake.point");
    const SUMMARY: CommandSummary =
        CommandSummary::from_static("Read one point. Needs the fake model loaded.");

    const TIMING: CommandTiming = CommandTiming::Immediate;

    fn availability(facts: &FakeFacts) -> CommandAvailability {
        if *facts.ready() {
            return CommandAvailability::Available;
        }
        CommandAvailability::Unavailable {
            code: UnavailableCode::MissingModel,
            note: RefusalNote::from_static("the fake host has loaded no point model"),
        }
    }

    fn register_handler(app: &mut App) {
        app.add_systems(Update, handle_fake_point.after(McpCommandSystems::Claim));
    }
}

fn handle_fake_point(
    facts: Res<FakeFacts>,
    mut queue: ResMut<PendingQueue<CommandCall<FakePoint>>>,
) {
    if queue.is_empty() {
        return;
    }
    for (args, responder) in take_calls::<FakePoint>(&mut queue) {
        responder.answer(&FakePointReply {
            point: args.point,
            level: facts.level(),
        });
    }
}
