use bevy::prelude::*;
use cobalt_mcp_host::{
    PendingQueue,
    command::McpCommand,
    dispatch::{CommandCall, McpCommandSystems, take_calls},
};
use cobalt_mcp_protocol::command::{
    CommandAvailability, CommandName, CommandSummary, CommandTiming, UnavailableCode,
};
use gdtf_battle_input::{InputSystems, LevelStep, dispatch_act_intents};
use serde::{Deserialize, Serialize};

use super::support::{NO_VIEW, ShownView, ViewChange, ViewControls};
use crate::dev::mcp::{
    commands::read::availability::battle_is_live,
    facts::GameFacts,
    wire::{cell::LevelNet, misc::ViewModeNet},
};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ViewLevelUpArgs {}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct ViewLevelUpReply {
    /// Storey the view is on after the step, read off the presenter's active level.
    level:     LevelNet,
    /// Whether every storey is drawn, read off the presenter's view mode.
    full_view: ViewModeNet,
}

pub(crate) struct ViewLevelUp;

impl McpCommand for ViewLevelUp {
    type Args = ViewLevelUpArgs;
    type Facts = GameFacts;
    type Parked = ();
    type Reply = ViewLevelUpReply;

    const NAME: CommandName = CommandName::from_static("view.level_up");
    const SUMMARY: CommandSummary = CommandSummary::from_static(
        "Raise the storey the battle view is drawn down to, the same path the level keys and the \
         action bar's level buttons take. Needs a running battle with its sim state loaded, but \
         not a caught-up screen — the view moves while the act log is still playing back. \
         Stepping past the top storey clamps, and the reply carries the storey the view is on \
         either way plus the unchanged view mode.",
    );
    const TIMING: CommandTiming = CommandTiming::Immediate;

    fn availability(facts: &GameFacts) -> CommandAvailability {
        battle_is_live(*facts)
    }

    fn register_handler(app: &mut App) {
        app.add_systems(
            Update,
            handle_view_level_up
                .in_set(InputSystems::Gather)
                .after(McpCommandSystems::Claim)
                .before(dispatch_act_intents),
        );
    }
}

fn handle_view_level_up(
    mut view: ViewControls,
    mut queue: ResMut<PendingQueue<CommandCall<ViewLevelUp>>>,
) {
    if queue.is_empty() {
        return;
    }
    for (_args, responder) in take_calls::<ViewLevelUp>(&mut queue) {
        let Some(ShownView { level, full_view }) = view.drive(ViewChange::Level(LevelStep::Up))
        else {
            responder.unavailable(UnavailableCode::MissingModel, NO_VIEW);
            continue;
        };
        responder.answer(&ViewLevelUpReply { level, full_view });
    }
}
