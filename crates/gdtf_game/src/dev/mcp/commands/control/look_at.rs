use bevy::prelude::*;
use cobalt_mcp_command::{
    command::McpCommand,
    dispatch::{CommandCall, DeferredReplies, McpCommandSystems, take_calls},
};
use cobalt_mcp_protocol::command::{
    CommandAvailability, CommandName, CommandSummary, CommandTiming, UnavailableCode,
};
use cobalt_mcp_transport::PendingQueue;
use gdtf_battle_presenter::{
    ActiveLevel, WorldCamera, cell_to_world, clamp_camera_to_bounds, set_camera_focus,
};
use serde::{Deserialize, Serialize};

use super::support::{NO_CAMERA, camera_cell};
use crate::dev::mcp::{
    commands::read::availability::battle_is_live, facts::GameFacts, wire::cell::CellLevelNet,
};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ViewLookAtArgs {
    /// Cell to centre on, as `battle.roster` and `battle.inspect` report cells.
    at: CellLevelNet,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct ViewLookAtReply {
    /// Cell the camera ended the frame centred on, absent when it sits off the grid.
    at: Option<CellLevelNet>,
}

pub(crate) struct ViewLookAt;

impl McpCommand for ViewLookAt {
    type Args = ViewLookAtArgs;
    type Facts = GameFacts;
    type Parked = ();
    type Reply = ViewLookAtReply;

    const NAME: CommandName = CommandName::from_static("view.look_at");
    const SUMMARY: CommandSummary = CommandSummary::from_static(
        "Centre the world camera on a cell, through the one function every camera mover writes \
         the transform with. It neither drives nor disturbs the once-per-battle framing on the \
         player's gangers. Needs a running battle with its sim state loaded. The reply is held \
         until the frame's bounds clamp has run and then names the cell the camera actually \
         ended on, which is the target unless the clamp pulled it back inside the map.",
    );
    const TIMING: CommandTiming = CommandTiming::Deferred;

    fn availability(facts: &GameFacts) -> CommandAvailability {
        battle_is_live(*facts)
    }

    fn register_handler(app: &mut App) {
        app.add_systems(
            Update,
            (
                claim_view_look_at
                    .after(McpCommandSystems::Claim)
                    .before(clamp_camera_to_bounds),
                settle_view_look_at.after(clamp_camera_to_bounds),
            ),
        );
    }
}

fn claim_view_look_at(
    mut queue: ResMut<PendingQueue<CommandCall<ViewLookAt>>>,
    mut deferred: ResMut<DeferredReplies<ViewLookAt>>,
    mut cameras: Query<&mut Transform, With<WorldCamera>>,
) {
    if queue.is_empty() {
        return;
    }
    for (args, responder) in take_calls::<ViewLookAt>(&mut queue) {
        let Some(mut transform) = cameras.iter_mut().next() else {
            responder.unavailable(UnavailableCode::MissingModel, NO_CAMERA);
            continue;
        };
        let (cell, level) = args.at.to_sim().split();
        let target = cell_to_world(cell, level);
        set_camera_focus(&mut transform, Vec2::new(target.x, target.y));
        deferred.park(responder, ());
    }
}

fn settle_view_look_at(
    mut deferred: ResMut<DeferredReplies<ViewLookAt>>,
    active: Option<Res<ActiveLevel>>,
    cameras: Query<&Transform, With<WorldCamera>>,
) {
    if deferred.is_empty() {
        return;
    }
    let reply = ViewLookAtReply {
        at: camera_cell(active.as_deref(), &cameras),
    };
    let delivered = deferred.answer_all(&reply);
    debug!(
        delivered = *delivered,
        "mcp: view.look_at answered once the frame's bounds clamp had run"
    );
}
