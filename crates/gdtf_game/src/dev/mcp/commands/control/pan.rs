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
    ActiveLevel, WorldCamera, cell_to_world, clamp_camera_to_bounds, pan_camera_by,
};
use gdtf_battle_sim::prelude::{Cell, Level};
use serde::{Deserialize, Serialize};

use super::support::{NO_CAMERA, NO_VIEW, camera_cell};
use crate::dev::mcp::{
    commands::read::availability::battle_is_live,
    facts::GameFacts,
    wire::cell::{CellLevelNet, CellNet},
};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ViewPanArgs {
    /// Signed cell offset to move by, in the same cell coordinates `battle.roster` reports.
    by: CellNet,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct ViewPanReply {
    /// Cell the camera ended the frame centred on, absent when it sits off the grid.
    at: Option<CellLevelNet>,
}

pub(crate) struct ViewPan;

impl McpCommand for ViewPan {
    type Args = ViewPanArgs;
    type Facts = GameFacts;
    type Parked = ();
    type Reply = ViewPanReply;

    const NAME: CommandName = CommandName::from_static("view.pan");
    const SUMMARY: CommandSummary = CommandSummary::from_static(
        "Move the world camera by a signed cell offset, through the one function every camera \
         mover writes the transform with. Unlike the pan keys, which move by speed times frame \
         time, the offset asked for is the offset taken, so a pan is repeatable. Needs a running \
         battle with its sim state loaded. The reply is held until the frame's bounds clamp has \
         run and then names the cell the camera actually ended on.",
    );
    const TIMING: CommandTiming = CommandTiming::Deferred;

    fn availability(facts: &GameFacts) -> CommandAvailability {
        battle_is_live(*facts)
    }

    fn register_handler(app: &mut App) {
        app.add_systems(
            Update,
            (
                claim_view_pan
                    .after(McpCommandSystems::Claim)
                    .before(clamp_camera_to_bounds),
                settle_view_pan.after(clamp_camera_to_bounds),
            ),
        );
    }
}

/// World delta of a signed cell offset, differenced against the origin cell on `level`.
fn world_delta(by: CellNet, level: Level) -> Vec2 {
    let origin = cell_to_world(Cell::new(0, 0), level);
    let offset = cell_to_world(Cell::new(*by.x, *by.y), level);
    Vec2::new(offset.x - origin.x, offset.y - origin.y)
}

fn claim_view_pan(
    mut queue: ResMut<PendingQueue<CommandCall<ViewPan>>>,
    mut deferred: ResMut<DeferredReplies<ViewPan>>,
    active: Option<Res<ActiveLevel>>,
    mut cameras: Query<&mut Transform, With<WorldCamera>>,
) {
    if queue.is_empty() {
        return;
    }
    for (args, responder) in take_calls::<ViewPan>(&mut queue) {
        let Some(level) = active.as_deref().map(|active| **active) else {
            responder.unavailable(UnavailableCode::MissingModel, NO_VIEW);
            continue;
        };
        let Some(mut transform) = cameras.iter_mut().next() else {
            responder.unavailable(UnavailableCode::MissingModel, NO_CAMERA);
            continue;
        };
        pan_camera_by(&mut transform, world_delta(args.by, level));
        deferred.park(responder, ());
    }
}

fn settle_view_pan(
    mut deferred: ResMut<DeferredReplies<ViewPan>>,
    active: Option<Res<ActiveLevel>>,
    cameras: Query<&Transform, With<WorldCamera>>,
) {
    if deferred.is_empty() {
        return;
    }
    let reply = ViewPanReply {
        at: camera_cell(active.as_deref(), &cameras),
    };
    let delivered = deferred.answer_all(&reply);
    debug!(
        delivered = *delivered,
        "mcp: view.pan answered once the frame's bounds clamp had run"
    );
}
