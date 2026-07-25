//! The editor's QA request-drain system set (GTW-804).

use bevy::prelude::*;

/// The editor's DEV QA request band — the editor's analogue of the game's
/// `InputSystems::Gather` (the `gdtf_battle_input` set the game's `net_qa` router and every
/// one of its consumers run in).
///
/// The editor depends on none of the game's crates, so it owns this set rather than borrowing
/// one. Everything that drains the QA inbox and dispatches to a handler goes in it, so later
/// children (GTW-805 / GTW-806 / GTW-808) hang their own editor-request consumers off ONE
/// named ordering point instead of chaining against a bare system name.
///
/// ## Where it runs, and why not in the egui pass
///
/// The set is configured in [`Update`], NOT in
/// [`EguiPrimaryContextPass`](bevy_egui::EguiPrimaryContextPass) (bevy-traps #8). Two reasons:
/// draining the inbox is not a UI system — it touches no egui context, so the rule that puts
/// `ctx_mut()` callers in that pass does not pull it there; and multipass runs the egui pass's
/// systems up to TWICE per frame, while answering a request is a one-shot side effect (each
/// [`Responder`](gdtf_net_qa_transport::Responder) writes one reply frame to the socket). A
/// once-per-frame `Update` drain keeps that side effect exactly-once without asking every
/// future editor handler to be idempotent.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EditorNetQaSystems {
    /// Drain the QA inbox and dispatch each request to its handler.
    Gather,
}
