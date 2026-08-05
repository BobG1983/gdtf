use bevy::{
    input_focus::{InputFocus, directional_navigation::DirectionalNavigationMap},
    prelude::*,
};
use gdtf_net_qa_transport::PendingQueue;
use gdtf_qa_command::{
    command::QaCommand,
    dispatch::{CommandCall, QaCommandSystems, take_calls},
};
use gdtf_qa_protocol::command::{CommandAvailability, CommandName, CommandSummary, CommandTiming};
use serde::{Deserialize, Serialize};

use crate::dev::net_qa::{facts::GameFacts, wire::token::FocusTargetNet};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct UiFocusArgs {}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct UiFocusReply {
    focused:   Option<FocusTargetNet>,
    focusable: Vec<FocusTargetNet>,
}

pub(crate) struct UiFocus;

impl QaCommand for UiFocus {
    type Args = UiFocusArgs;
    type Facts = GameFacts;
    type Reply = UiFocusReply;

    const NAME: CommandName = CommandName::from_static("ui.focus");
    const SUMMARY: CommandSummary = CommandSummary::from_static(
        "Read which widget holds keyboard and gamepad focus, plus every widget the current \
         screen registered as focusable. Both resources exist from app build, so this answers \
         on any screen; the focusable list is whatever topology that screen registered.",
    );
    const TIMING: CommandTiming = CommandTiming::Immediate;

    fn availability(_facts: &GameFacts) -> CommandAvailability {
        CommandAvailability::Available
    }

    fn register_handler(app: &mut App) {
        app.add_systems(Update, handle_ui_focus.after(QaCommandSystems::Claim));
    }
}

fn handle_ui_focus(
    focus: Res<InputFocus>,
    nav_map: Res<DirectionalNavigationMap>,
    mut queue: ResMut<PendingQueue<CommandCall<UiFocus>>>,
) {
    if queue.is_empty() {
        return;
    }
    let focused = focus.get().map(token_for);
    let mut registered: Vec<Entity> = nav_map.neighbors.keys().copied().collect();
    registered.sort_unstable();
    let focusable: Vec<FocusTargetNet> = registered.into_iter().map(token_for).collect();
    for (_args, responder) in take_calls::<UiFocus>(&mut queue) {
        responder.answer(&UiFocusReply {
            focused,
            focusable: focusable.clone(),
        });
    }
}

const fn token_for(entity: Entity) -> FocusTargetNet {
    FocusTargetNet::new(entity.to_bits())
}
