//! The editor's offscreen capture-target present path (GTW-918) — retarget the editor's egui
//! camera to an offscreen `Image` the render graph writes every tick, then blit that image back
//! to the window.
//!
//! ## Why this exists
//!
//! The capture pump ([`super::screenshot`]) shipped reading the window swapchain via
//! `Screenshot::primary_window`. That path DOES work while the editor window is visible — it was
//! observed live on 2026-07-29 returning a fully rendered egui shell. What it depends on is
//! window VISIBILITY: per GTW-764, on macOS a backgrounded, occluded or minimized window's Metal
//! drawable is stale, so the same copy comes back BLACK. An unattended agent-QA run launches the
//! editor as a child process and then works elsewhere, so nothing keeps that window frontmost —
//! which is exactly the condition this removes the dependency on.
//!
//! Capturing an offscreen `Image` instead (`Screenshot::image`) is independent of window
//! presentation: the render graph writes that image every tick. A dedicated present camera blits
//! it back to the window, so what a developer sees on the window and what the pump captures are
//! the same pixels by construction.
//!
//! ## The egui difference from the game's GTW-764 path
//!
//! The game retargets a `bevy_ui` tree and had to add `IsDefaultUiCamera` so its present camera
//! did not hijack the default-UI-camera binding. egui needs no analogue and MUST NOT get one:
//! its context is a component on the CAMERA ENTITY, so retargeting that entity moves egui's
//! output with it. `bevy_egui`'s `egui_pass` resolves the camera's `ViewTarget` and never
//! inspects `RenderTarget`, and the pass is registered into the `Core2d`/`Core3d` graphs, which
//! run for image-targeted cameras exactly as for window ones.
//!
//! What DOES need care is input: `bevy_egui` records a context's window mapping once, on
//! `Added<EguiContext>`, and only for a window-targeted context. [`retarget`] gates on that
//! mapping existing — see its system doc for the whole trap.
//!
//! ## Fixed window size
//!
//! The offscreen image is created once at the primary window's physical size. The QA harness
//! uses a FIXED window size, so window resize is NOT handled — the image keeps its original
//! size, exactly as the game's path does.
//!
//! ## Members (one concern per file, per module-layout)
//!
//! - [`target`] — the offscreen [`EditorQaCaptureTarget`](target::EditorQaCaptureTarget) image
//!   (created at window size, with `COPY_SRC` added so `Screenshot::image` can read it back),
//!   and the [`EditorShotSource`](super::screenshot::EditorShotSource) that names it.
//! - [`retarget`] — inserts `RenderTarget::Image` on the editor's egui camera, at the window's
//!   real scale factor, behind the `bevy_egui` input-mapping gate.
//! - [`blit`] — the present camera + full-window sprite that shows the image on the window.
//! - [`plugin`] — [`EditorCapturePresentPlugin`](plugin::EditorCapturePresentPlugin): the
//!   `WinitSettings` continuous override + the three `Update` systems, chained.
//! - `test` — the headless pins: the `WinitSettings` override, the target's size / format /
//!   `COPY_SRC`, the retarget's mapping gate and scale factor, the present camera, the egui
//!   context's ownership, and the schedule placement.

mod blit;
mod plugin;
mod retarget;
mod target;

#[cfg(test)]
mod test;

// Consumed OUTSIDE `present`: the plugin by the editor `net_qa` wiring (`super::plugin`), and
// the offscreen target plus its whole-target comparison by the capture pump's pre-spawn
// consistency check (`super::screenshot::aim`, GTW-922).
pub(in crate::net_qa) use plugin::EditorCapturePresentPlugin;
pub(in crate::net_qa) use target::{EditorQaCaptureTarget, aims_at};
