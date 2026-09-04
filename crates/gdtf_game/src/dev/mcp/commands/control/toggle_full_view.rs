use bevy::prelude::*;
use cobalt_mcp_command::{
    command::QaCommand,
    dispatch::{CommandCall, QaCommandSystems, take_calls},
};
use cobalt_mcp_protocol::command::{
    CommandAvailability, CommandName, CommandSummary, CommandTiming, UnavailableCode,
};
use cobalt_mcp_transport::PendingQueue;
use gdtf_battle_input::{InputSystems, dispatch_act_intents};
use serde::{Deserialize, Serialize};

use super::support::{NO_VIEW, ShownView, ViewChange, ViewControls};
use crate::dev::mcp::{
    commands::read::availability::battle_is_live,
    facts::GameFacts,
    wire::{cell::LevelNet, misc::ViewModeNet},
};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ViewToggleFullViewArgs {}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct ViewToggleFullViewReply {
    /// Storey the view is on, which a toggle leaves alone.
    level:     LevelNet,
    /// Whether every storey is drawn after the flip.
    full_view: ViewModeNet,
}

pub(crate) struct ViewToggleFullView;

impl QaCommand for ViewToggleFullView {
    type Args = ViewToggleFullViewArgs;
    type Facts = GameFacts;
    type Parked = ();
    type Reply = ViewToggleFullViewReply;

    const NAME: CommandName = CommandName::from_static("view.toggle_full_view");
    const SUMMARY: CommandSummary = CommandSummary::from_static(
        "Flip the battle view between showing storeys down to the active one and showing every \
         storey, the same path the full-view key takes. Needs a running battle with its sim \
         state loaded, but not a caught-up screen. The reply carries the view mode after the \
         flip and the storey it left alone.",
    );
    const TIMING: CommandTiming = CommandTiming::Immediate;

    fn availability(facts: &GameFacts) -> CommandAvailability {
        battle_is_live(*facts)
    }

    fn register_handler(app: &mut App) {
        app.add_systems(
            Update,
            handle_view_toggle_full_view
                .in_set(InputSystems::Gather)
                .after(QaCommandSystems::Claim)
                .before(dispatch_act_intents),
        );
    }
}

fn handle_view_toggle_full_view(
    mut view: ViewControls,
    mut queue: ResMut<PendingQueue<CommandCall<ViewToggleFullView>>>,
) {
    if queue.is_empty() {
        return;
    }
    for (_args, responder) in take_calls::<ViewToggleFullView>(&mut queue) {
        let Some(ShownView { level, full_view }) = view.drive(ViewChange::FullView) else {
            responder.unavailable(UnavailableCode::MissingModel, NO_VIEW);
            continue;
        };
        responder.answer(&ViewToggleFullViewReply { level, full_view });
    }
}
