use bevy::{
    ecs::message::Messages,
    prelude::*,
    window::{CursorMoved, PrimaryWindow},
};
use gdtf_battle_input::pick_hovered_cell;
use gdtf_net_qa_transport::PendingQueue;
use gdtf_qa_command::{
    command::QaCommand,
    dispatch::{CommandCall, QaCommandSystems, take_calls},
};
use gdtf_qa_protocol::command::{
    CommandAvailability, CommandName, CommandSummary, CommandTiming, RefusalNote, UnavailableCode,
};
use serde::{Deserialize, Serialize};

use crate::dev::net_qa::{facts::GameFacts, wire::pointer::PointerPosNet};

/// The refusal a hover gets on a host that never opened a window.
const NO_WINDOW: RefusalNote = RefusalNote::from_static(
    "a pixel hover is written into the primary Window's cursor and announced as a cursor move, \
     and this host carries no primary Window to write it into",
);

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct InputHoverArgs {
    /// Where to put the pointer, in logical window pixels.
    at: PointerPosNet,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct InputHoverReply {
    /// The pixel the pointer was moved to.
    at: PointerPosNet,
}

pub(crate) struct InputHover;

impl QaCommand for InputHover {
    type Args = InputHoverArgs;
    type Facts = GameFacts;
    type Parked = ();
    type Reply = InputHoverReply;

    const NAME: CommandName = CommandName::from_static("input.hover");
    const SUMMARY: CommandSummary = CommandSummary::from_static(
        "Move the pointer to a pixel the way the mouse does: the cursor is written into the \
         primary window and a CursorMoved message goes out, which also takes pointer ownership \
         back from the gamepad. The reply echoes the pixel and never a cell — projecting a pixel \
         onto a cell belongs to the picking system, so read the resulting cell with \
         battle.selection.",
    );
    const TIMING: CommandTiming = CommandTiming::Immediate;

    fn availability(_facts: &GameFacts) -> CommandAvailability {
        CommandAvailability::Available
    }

    fn register_handler(app: &mut App) {
        app.add_systems(
            Update,
            handle_input_hover
                .after(QaCommandSystems::Claim)
                .before(pick_hovered_cell),
        );
    }
}

pub(super) fn handle_input_hover(
    mut queue: ResMut<PendingQueue<CommandCall<InputHover>>>,
    mut windows: Query<(Entity, &mut Window), With<PrimaryWindow>>,
    mut moved: Option<ResMut<Messages<CursorMoved>>>,
) {
    if queue.is_empty() {
        return;
    }
    for (args, responder) in take_calls::<InputHover>(&mut queue) {
        let (Ok((entity, mut window)), Some(moved)) = (windows.single_mut(), moved.as_deref_mut())
        else {
            responder.unavailable(UnavailableCode::WrongState, NO_WINDOW);
            continue;
        };
        let at = Vec2::new(f32::from(*args.at.x), f32::from(*args.at.y));
        let delta = window.cursor_position().map(|was| at - was);
        window.set_cursor_position(Some(at));
        let _written = moved.write(CursorMoved {
            window: entity,
            position: at,
            delta,
        });
        responder.answer(&InputHoverReply { at: args.at });
    }
}
