//! The prefab **preview render machinery** (GTW-515 C4.3) — the render-to-texture VIEWPORT the
//! egui PREFAB mode draws: an offscreen [`Image`](bevy::image::Image) render target, a dedicated
//! second [`Camera2d`](bevy::prelude::Camera2d) that renders the prefab preview tiles into it on an
//! ISOLATED [`RenderLayers`](bevy::camera::visibility::RenderLayers), and the change-driven tile
//! redraw + the once-per-frame set-to-target zoom/pan apply.
//!
//! Wiring-only module. The render target + camera + the multipass-idempotent apply live in
//! [`target`]; the change-driven tile-sprite redraw + hover ghost in [`tiles`]; the owned zoom/pan
//! view state + the PURE cursor-anchored-zoom math in [`view`]; and the PURE UV↔cell↔world
//! coordinate mapping in [`coords`].
//!
//! ## Isolation approach: `RenderLayers` (the research-recommended primary path)
//!
//! Shipped with the two-camera [`RenderLayers::layer`](bevy::camera::visibility::RenderLayers::layer)`(1)` split (the offscreen preview camera +
//! every preview tile on layer 1, the editor's window camera + everything else on layer 0), exactly
//! the shipping `bevy_egui` `render_to_image_widget.rs` pattern — known-good on Bevy 0.19 /
//! `bevy_egui` 0.41. The editor's existing (auto-created) primary egui context is untouched; the
//! preview is just a texture drawn inside it. The F1 single-shared-camera fallback (C4.12) was NOT
//! needed.
//!
//! The lifecycle is state-scoped (bevy-traps #1): the target + camera spawn `OnEnter(Editing)` and
//! despawn `OnExit(Editing)`; the owned [`PreviewPan`](view::PreviewPan) target is inserted /
//! removed by the plugin's `init_state_scoped_resource` registration (GTW-575, the shared
//! `gdtf_state_scoped` seam) alongside the kept [`CanvasZoom`](crate::canvas::CanvasZoom).

pub(crate) mod coords;
mod register;
pub(crate) mod target;
mod tiles;
pub(crate) mod view;

pub(crate) use register::register_preview;
