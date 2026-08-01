//! The transport-half registration: the capture queue, the pump's cross-frame state, and
//! the always-on router (GTW-736, cut to the command layer by GTW-943).
//!
//! The queue type is the shared transport's ([`gdtf_net_qa_transport`], GTW-803); this file
//! is where THIS host instantiates it, in the ordering the router needs.

use bevy::prelude::*;
use gdtf_battle_input::InputSystems;
use gdtf_net_qa_transport::PendingQueue;
use gdtf_qa_command::dispatch::QaCommandSystems;

use crate::dev::net_qa::{
    commands::register_game_commands,
    router::route_requests,
    screenshot::{InFlightShots, QaShotDir, ScreenshotPayload, ShotPollBudget, ShotSequence},
};

/// Init the capture queue plus the pump's cross-frame state, and register the always-on
/// router in the [`InputSystems::Gather`] band.
pub(super) fn register_transport(app: &mut App) {
    app.init_resource::<PendingQueue<ScreenshotPayload>>()
        // GTW-740 — the capture pump's own state: the captures in-flight across frames, the
        // poll budget, the confinement directory, and the monotonic sequence that makes
        // every capture's output path unique.
        .init_resource::<InFlightShots>()
        .init_resource::<ShotPollBudget>()
        .init_resource::<QaShotDir>()
        .init_resource::<ShotSequence>();
    // The capture queue has NO deadline sweep: the pump claims every capture the frame it is
    // queued and owns its own multi-frame poll timeout, so a generic sweep would only ever
    // answer the wrong reply for it.
    app.add_systems(
        Update,
        // GTW-942 — the router IS the command layer's `Route` band, because it is the ONE
        // system that drains the inbox: the `Catalogue` and `Run` arms live inside it, so
        // the decode steps `register_game_commands` wires are ordered after THIS system
        // rather than after a second router that must never exist.
        route_requests
            .in_set(QaCommandSystems::Route)
            .in_set(InputSystems::Gather),
    );
    register_game_commands(app);
}
