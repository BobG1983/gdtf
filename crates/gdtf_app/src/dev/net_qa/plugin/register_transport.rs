use bevy::prelude::*;
use gdtf_qa_command::dispatch::QaCommandSystems;

use crate::dev::net_qa::{commands::register_game_commands, router::route_requests};

pub(super) fn register_transport(app: &mut App) {
    app.add_systems(Update, route_requests.in_set(QaCommandSystems::Route));
    register_game_commands(app);
}
