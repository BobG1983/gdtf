use bevy::prelude::*;
use cobalt_mcp_host::dispatch::McpCommandSystems;

use crate::dev::mcp::{commands::register_game_commands, router::route_requests};

pub(super) fn register_transport(app: &mut App) {
    app.add_systems(Update, route_requests.in_set(McpCommandSystems::Route));
    register_game_commands(app);
}
