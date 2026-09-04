use bevy::{input_focus::InputFocus, prelude::*};
use cobalt_mcp_host::{
    PendingQueue,
    command::McpCommand,
    dispatch::{CommandCall, DeferredReplies, McpCommandSystems, take_calls},
};
use cobalt_mcp_protocol::command::{
    CommandAvailability, CommandName, CommandSummary, CommandTiming, RefusalNote, UnavailableCode,
};
use gdtf_ui::focus_nav::{FocusActivated, FocusNavSystems};

use super::support::{FocusReply, settle_focus};
use crate::dev::mcp::{commands::act::support::NoArgs, facts::GameFacts};

/// The refusal an activation gets when no widget holds focus.
const NOTHING_FOCUSED: RefusalNote = RefusalNote::from_static(
    "activation always lands on whatever holds focus, and nothing does — set focus with \
     `input.set_focus` first",
);

pub(crate) struct InputActivate;

impl McpCommand for InputActivate {
    type Args = NoArgs;
    type Facts = GameFacts;
    type Parked = ();
    type Reply = FocusReply;

    const NAME: CommandName = CommandName::from_static("input.activate");
    const SUMMARY: CommandSummary = CommandSummary::from_static(
        "Activate the widget that holds focus, writing the same message Enter and the gamepad's \
         south button write. It never takes a target: focus one widget with `input.set_focus` or \
         `input.focus_step` and then activate it. The reply reports the focus the frame settled \
         on, which a screen change may already have moved.",
    );
    const TIMING: CommandTiming = CommandTiming::Immediate;

    fn availability(_facts: &GameFacts) -> CommandAvailability {
        CommandAvailability::Available
    }

    fn register_handler(app: &mut App) {
        app.add_systems(
            Update,
            (
                claim_input_activate
                    .in_set(FocusNavSystems::Bridge)
                    .after(McpCommandSystems::Claim),
                settle_input_activate.after(FocusNavSystems::Apply),
            ),
        );
    }
}

fn claim_input_activate(
    mut queue: ResMut<PendingQueue<CommandCall<InputActivate>>>,
    mut deferred: ResMut<DeferredReplies<InputActivate>>,
    focus: Res<InputFocus>,
    mut activated: MessageWriter<FocusActivated>,
) {
    if queue.is_empty() {
        return;
    }
    for (_args, responder) in take_calls::<InputActivate>(&mut queue) {
        let Some(focused) = focus.get() else {
            responder.unavailable(UnavailableCode::WrongState, NOTHING_FOCUSED);
            continue;
        };
        activated.write(FocusActivated::new(focused));
        deferred.park(responder, ());
    }
}

fn settle_input_activate(
    focus: Res<InputFocus>,
    mut deferred: ResMut<DeferredReplies<InputActivate>>,
) {
    settle_focus::<InputActivate>(&focus, &mut deferred);
}
