use bevy::prelude::*;
use gdtf_battle_input::InputSystems;
use gdtf_net_qa_transport::PendingQueue;
use gdtf_qa_command::dispatch::QaCommandSystems;

use crate::dev::net_qa::{
    commands::register_game_commands,
    router::route_requests,
    screenshot::{InFlightShots, QaShotDir, ScreenshotPayload, ShotPollBudget, ShotSequence},
};

pub(super) fn register_transport(app: &mut App) {
    app.init_resource::<PendingQueue<ScreenshotPayload>>()
        .init_resource::<InFlightShots>()
        .init_resource::<ShotPollBudget>()
        .init_resource::<QaShotDir>()
        .init_resource::<ShotSequence>();
    app.add_systems(
        Update,
        route_requests
            .in_set(QaCommandSystems::Route)
            .in_set(InputSystems::Gather),
    );
    register_game_commands(app);
}
