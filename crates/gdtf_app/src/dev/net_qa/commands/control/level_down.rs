use bevy::prelude::*;
use gdtf_battle_input::{InputSystems, LevelStep, dispatch_act_intents};
use gdtf_net_qa_transport::PendingQueue;
use gdtf_qa_command::{
    command::QaCommand,
    dispatch::{CommandCall, QaCommandSystems, take_calls},
};
use gdtf_qa_protocol::command::{
    CommandAvailability, CommandName, CommandSummary, CommandTiming, UnavailableCode,
};
use serde::{Deserialize, Serialize};

use super::support::{NO_VIEW, ShownView, ViewChange, ViewControls};
use crate::dev::net_qa::{
    commands::read::availability::battle_is_live,
    facts::GameFacts,
    wire::{cell::LevelNet, misc::ViewModeNet},
};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ViewLevelDownArgs {}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct ViewLevelDownReply {
    /// Storey the view is on after the step, read off the presenter's active level.
    level:     LevelNet,
    /// Whether every storey is drawn, read off the presenter's view mode.
    full_view: ViewModeNet,
}

pub(crate) struct ViewLevelDown;

impl QaCommand for ViewLevelDown {
    type Args = ViewLevelDownArgs;
    type Facts = GameFacts;
    type Parked = ();
    type Reply = ViewLevelDownReply;

    const NAME: CommandName = CommandName::from_static("view.level_down");
    const SUMMARY: CommandSummary = CommandSummary::from_static(
        "Lower the storey the battle view is drawn down to, the same path the level keys and the \
         action bar's level buttons take. Needs a running battle with its sim state loaded, but \
         not a caught-up screen — the view moves while the act log is still playing back. \
         Stepping below the ground floor clamps, and the reply carries the storey the view is on \
         either way plus the unchanged view mode.",
    );
    const TIMING: CommandTiming = CommandTiming::Immediate;

    fn availability(facts: &GameFacts) -> CommandAvailability {
        battle_is_live(*facts)
    }

    fn register_handler(app: &mut App) {
        app.add_systems(
            Update,
            handle_view_level_down
                .in_set(InputSystems::Gather)
                .after(QaCommandSystems::Claim)
                .before(dispatch_act_intents),
        );
    }
}

fn handle_view_level_down(
    mut view: ViewControls,
    mut queue: ResMut<PendingQueue<CommandCall<ViewLevelDown>>>,
) {
    if queue.is_empty() {
        return;
    }
    for (_args, responder) in take_calls::<ViewLevelDown>(&mut queue) {
        let Some(ShownView { level, full_view }) = view.drive(ViewChange::Level(LevelStep::Down))
        else {
            responder.unavailable(UnavailableCode::MissingModel, NO_VIEW);
            continue;
        };
        responder.answer(&ViewLevelDownReply { level, full_view });
    }
}
