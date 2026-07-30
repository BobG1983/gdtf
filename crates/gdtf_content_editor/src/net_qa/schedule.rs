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
///
/// ## Why there are two bands, and why [`Present`](Self::Present) runs first
///
/// The offscreen present path WRITES the editor camera's
/// [`RenderTarget`](bevy::camera::RenderTarget) (its retarget inserts
/// `RenderTarget::Image`), and the capture pump in [`Gather`](Self::Gather) READS that same
/// component to decide whether the pixels a capture would read are the pixels the UI camera
/// writes. Without an explicit ordering those two are ambiguous (bevy-traps #3): on the frame
/// the retarget's deferred insert lands, the pump would see either the old
/// `RenderTarget::Window` or the new `RenderTarget::Image` depending on how the executor
/// happened to schedule them, so a screenshot settling on that frame would be captured or
/// refused at random. Bevy also only guarantees one system's deferred
/// [`Commands`](bevy::prelude::Commands) are applied before another's run when there IS an
/// explicit ordering between them (`Schedule::apply_deferred`'s own doc).
///
/// `Present` therefore runs BEFORE `Gather`, configured by
/// `EditorCapturePresentPlugin` so the constraint travels with the systems it constrains.
/// The write-then-read direction is the useful one: a capture requested on the retarget frame
/// reads the target the camera was just aimed at, instead of being refused for a mismatch that
/// the same frame already resolved.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EditorNetQaSystems {
    /// Create the offscreen capture target, aim the editor's egui camera at it, and put the
    /// present camera on the window — everything the capture pump's target read depends on.
    Present,
    /// Drain the QA inbox and dispatch each request to its handler.
    Gather,
}
