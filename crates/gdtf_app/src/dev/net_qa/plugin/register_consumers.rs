//! The per-request consumer registrations and their ordering (GTW-740, cut to the capture
//! pump by GTW-943).

use bevy::prelude::*;
use gdtf_battle_input::InputSystems;

use crate::dev::net_qa::{router::route_requests, screenshot::drive_screenshots};

/// Register every consumer system (see the per-block comments for each one's ordering and
/// run condition).
pub(super) fn register_consumers(app: &mut App) {
    // GTW-740 — the capture pump. In the `InputSystems::Gather` band ordered
    // `.after(route_requests)` so it claims a capture queued the SAME frame, then holds it
    // across frames while the GPU readback flushes the PNG (the reply is genuinely deferred
    // — see `drive_screenshots`). Runs unconditionally: a capture needs no live battle.
    app.add_systems(
        Update,
        drive_screenshots
            .in_set(InputSystems::Gather)
            .after(route_requests),
    );
}
