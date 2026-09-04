use bevy::{input_focus::InputFocus, prelude::*};
use cobalt_mcp_command::{
    command::McpCommand,
    dispatch::{CommandCall, DeferredReplies, McpCommandSystems, take_calls},
};
use cobalt_mcp_protocol::command::{
    CommandAvailability, CommandName, CommandSummary, CommandTiming,
};
use cobalt_mcp_transport::PendingQueue;
use gdtf_ui::focus_nav::{FocusNavSystems, NavDirection, NavigateRequest};
use serde::Deserialize;

use super::support::{FocusReply, settle_focus};
use crate::dev::mcp::{facts::GameFacts, wire::key::FocusStepNet};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct InputFocusStepArgs {
    /// Which way to step the focus.
    step: FocusStepNet,
}

pub(crate) struct InputFocusStep;

impl McpCommand for InputFocusStep {
    type Args = InputFocusStepArgs;
    type Facts = GameFacts;
    type Parked = ();
    type Reply = FocusReply;

    const NAME: CommandName = CommandName::from_static("input.focus_step");
    const SUMMARY: CommandSummary = CommandSummary::from_static(
        "Step focus one widget along the screen's navigation map, writing the same navigate \
         request the arrow keys and the d-pad write. Next and Prev are the down and up edges; \
         Left and Right are the west and east ones. A step with no neighbour that way is \
         swallowed exactly as the real one is, so compare the focus in the reply against the \
         focus before the call to see whether it moved.",
    );
    const TIMING: CommandTiming = CommandTiming::Immediate;

    fn availability(_facts: &GameFacts) -> CommandAvailability {
        CommandAvailability::Available
    }

    fn register_handler(app: &mut App) {
        app.add_systems(
            Update,
            (
                claim_input_focus_step
                    .in_set(FocusNavSystems::Bridge)
                    .after(McpCommandSystems::Claim),
                settle_input_focus_step.after(FocusNavSystems::Apply),
            ),
        );
    }
}

pub(super) const fn direction(step: FocusStepNet) -> NavDirection {
    match step {
        FocusStepNet::Next => NavDirection::DOWN,
        FocusStepNet::Prev => NavDirection::UP,
        FocusStepNet::Left => NavDirection::WEST,
        FocusStepNet::Right => NavDirection::EAST,
    }
}

fn claim_input_focus_step(
    mut queue: ResMut<PendingQueue<CommandCall<InputFocusStep>>>,
    mut deferred: ResMut<DeferredReplies<InputFocusStep>>,
    mut navigate: MessageWriter<NavigateRequest>,
) {
    if queue.is_empty() {
        return;
    }
    for (args, responder) in take_calls::<InputFocusStep>(&mut queue) {
        navigate.write(NavigateRequest::new(direction(args.step)));
        deferred.park(responder, ());
    }
}

fn settle_input_focus_step(
    focus: Res<InputFocus>,
    mut deferred: ResMut<DeferredReplies<InputFocusStep>>,
) {
    settle_focus::<InputFocusStep>(&focus, &mut deferred);
}
