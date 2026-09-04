use bevy::{
    input_focus::{FocusCause, InputFocus, directional_navigation::DirectionalNavigationMap},
    prelude::*,
};
use cobalt_mcp_command::{
    command::McpCommand,
    dispatch::{CommandCall, DeferredReplies, McpCommandSystems, take_calls},
};
use cobalt_mcp_protocol::command::{
    CommandAvailability, CommandName, CommandSummary, CommandTiming, RefusalNote, UnavailableCode,
};
use cobalt_mcp_transport::PendingQueue;
use gdtf_ui::focus_nav::FocusNavSystems;
use serde::Deserialize;

use super::support::{FocusReply, settle_focus};
use crate::dev::mcp::{facts::GameFacts, wire::token::FocusTargetNet};

/// The refusal a token that names no registered widget gets.
const NOT_FOCUSABLE: RefusalNote = RefusalNote::from_static(
    "focus is only set on a widget the current screen registered as focusable — name one of the \
     targets `ui.focus` reports in its focusable list",
);

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct InputSetFocusArgs {
    /// Widget to focus, as `ui.focus` reports focusable targets.
    target: FocusTargetNet,
}

pub(crate) struct InputSetFocus;

impl McpCommand for InputSetFocus {
    type Args = InputSetFocusArgs;
    type Facts = GameFacts;
    type Parked = ();
    type Reply = FocusReply;

    const NAME: CommandName = CommandName::from_static("input.set_focus");
    const SUMMARY: CommandSummary = CommandSummary::from_static(
        "Put keyboard and gamepad focus on one widget, named by a token from `ui.focus`'s \
         focusable list. A token the current screen never registered is refused rather than \
         focused, so focus can never land somewhere the navigation map cannot leave. The reply \
         reports the focus the frame settled on.",
    );
    const TIMING: CommandTiming = CommandTiming::Immediate;

    fn availability(_facts: &GameFacts) -> CommandAvailability {
        CommandAvailability::Available
    }

    fn register_handler(app: &mut App) {
        app.add_systems(
            Update,
            (
                claim_input_set_focus
                    .in_set(FocusNavSystems::Bridge)
                    .after(McpCommandSystems::Claim),
                settle_input_set_focus.after(FocusNavSystems::Apply),
            ),
        );
    }
}

fn claim_input_set_focus(
    mut queue: ResMut<PendingQueue<CommandCall<InputSetFocus>>>,
    mut deferred: ResMut<DeferredReplies<InputSetFocus>>,
    mut focus: ResMut<InputFocus>,
    nav_map: Res<DirectionalNavigationMap>,
) {
    if queue.is_empty() {
        return;
    }
    for (args, responder) in take_calls::<InputSetFocus>(&mut queue) {
        let registered = Entity::try_from_bits(*args.target)
            .filter(|entity| nav_map.neighbors.contains_key(entity));
        let Some(target) = registered else {
            responder.unavailable(UnavailableCode::WrongState, NOT_FOCUSABLE);
            continue;
        };
        focus.set(target, FocusCause::Navigated);
        deferred.park(responder, ());
    }
}

fn settle_input_set_focus(
    focus: Res<InputFocus>,
    mut deferred: ResMut<DeferredReplies<InputSetFocus>>,
) {
    settle_focus::<InputSetFocus>(&focus, &mut deferred);
}
