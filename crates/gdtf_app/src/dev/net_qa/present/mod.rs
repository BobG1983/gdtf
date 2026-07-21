//! The offscreen capture-target present path (GTW-764) — retarget the game cameras to an
//! offscreen `Image` the render graph writes every tick, then blit that image back to the
//! window.
//!
//! ## Why this exists
//!
//! The T7 capture pump ([`super::screenshot`]) originally read the window swapchain via
//! `Screenshot::primary_window`. On macOS a backgrounded window's Metal drawable is stale, so
//! a capture of an unfocused / occluded window came back BLACK — and the QA harness runs the
//! game backgrounded essentially always. Capturing an offscreen `Image` instead
//! (`Screenshot::image`) is independent of window presentation: the render graph writes that
//! image every tick regardless of focus. Because a `bevy_ui` node tree renders to exactly ONE
//! camera, the WHOLE game (world + UI/HUD) is retargeted to the image and a dedicated present
//! camera blits it back to the window — so what a developer sees on the window and what the
//! pump captures are the same pixels by construction.
//!
//! ## Fixed window size (C6)
//!
//! The offscreen image is created once at the primary window's physical size. The QA harness
//! uses a FIXED window size, so window resize is NOT handled — the image keeps its original
//! size. (Recreating the target on `WindowResized` is a future extension; a resize would
//! otherwise leave the present blit letterboxed / cropped.)
//!
//! ## Members (one concern per file, per module-layout)
//!
//! - [`target`] — the offscreen [`QaCaptureTarget`](target::QaCaptureTarget) image (created at
//!   window size, with `COPY_SRC` added so `Screenshot::image` can read it back).
//! - [`retarget`] — inserts `RenderTarget::Image` on the world + UI cameras.
//! - [`blit`] — the present camera + full-window sprite that shows the image on the window.
//! - [`plugin`] — [`CapturePresentPlugin`](plugin::CapturePresentPlugin): the `WinitSettings`
//!   continuous override + the three systems.

mod blit;
mod plugin;
mod retarget;
mod target;

#[cfg(test)]
mod test;

// Consumed OUTSIDE `present`: the plugin by `net_qa`'s wiring (`super::plugin`), and the
// capture-target resource by the T7 screenshot pump + the T15 `screenshot_after` child (both
// pick `Screenshot::image` over `primary_window` when it exists).
pub(in crate::dev::net_qa) use plugin::CapturePresentPlugin;
pub(in crate::dev::net_qa) use target::QaCaptureTarget;
