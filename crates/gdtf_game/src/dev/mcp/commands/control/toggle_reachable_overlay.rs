use bevy::prelude::*;
use cobalt_mcp_host::{
    PendingQueue,
    command::McpCommand,
    dispatch::{CommandCall, McpCommandSystems, take_calls},
};
use cobalt_mcp_protocol::command::{
    CommandAvailability, CommandName, CommandSummary, CommandTiming, RefusalNote, UnavailableCode,
};
use gdtf_battle_input::{InputSystems, dispatch_act_intents};
use gdtf_battle_presenter::ReachableOverlayEnabled;
use serde::{Deserialize, Serialize};

use crate::dev::mcp::{
    commands::read::availability::battle_is_live, facts::GameFacts, wire::misc::ReachableOverlayNet,
};

/// The refusal this command gets when the battlescape renderer has not put the flag up.
const NO_OVERLAY_FLAG: RefusalNote = RefusalNote::from_static(
    "the reachable-range overlay flag is put up with the battlescape renderer, and it is not up \
     right now",
);

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ViewToggleReachableOverlayArgs {}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct ViewToggleReachableOverlayReply {
    /// Whether the overlay is on after the flip.
    overlay: ReachableOverlayNet,
}

pub(crate) struct ViewToggleReachableOverlay;

impl McpCommand for ViewToggleReachableOverlay {
    type Args = ViewToggleReachableOverlayArgs;
    type Facts = GameFacts;
    type Parked = ();
    type Reply = ViewToggleReachableOverlayReply;

    const NAME: CommandName = CommandName::from_static("view.toggle_reachable_overlay");
    const SUMMARY: CommandSummary = CommandSummary::from_static(
        "Flip the reachable-range debug overlay, which tints the cells the selected ganger can \
         walk to. Needs a running battle with its sim state loaded, but not a caught-up screen. \
         The reply carries whether the overlay is on after the flip. A release build keeps the \
         flag but draws nothing.",
    );
    const TIMING: CommandTiming = CommandTiming::Immediate;

    fn availability(facts: &GameFacts) -> CommandAvailability {
        battle_is_live(*facts)
    }

    fn register_handler(app: &mut App) {
        app.add_systems(
            Update,
            handle_view_toggle_reachable_overlay
                .in_set(InputSystems::Gather)
                .after(McpCommandSystems::Claim)
                .before(dispatch_act_intents),
        );
    }
}

fn handle_view_toggle_reachable_overlay(
    mut enabled: Option<ResMut<ReachableOverlayEnabled>>,
    mut queue: ResMut<PendingQueue<CommandCall<ViewToggleReachableOverlay>>>,
) {
    if queue.is_empty() {
        return;
    }
    for (_args, responder) in take_calls::<ViewToggleReachableOverlay>(&mut queue) {
        let Some(flag) = enabled.as_mut() else {
            responder.unavailable(UnavailableCode::MissingModel, NO_OVERLAY_FLAG);
            continue;
        };
        let flipped = flag.flipped();
        **flag = flipped;
        responder.answer(&ViewToggleReachableOverlayReply {
            overlay: ReachableOverlayNet::from_presenter(flipped),
        });
    }
}
