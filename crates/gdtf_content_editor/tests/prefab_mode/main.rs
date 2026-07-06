//! Headless integration test for the GTW-515 (C4) egui PREFAB mode + render-to-texture viewport.
//!
//! Drives the REAL [`MapEditorPlugin`] on the no-renderer `DefaultPlugins` UI harness (a live
//! [`AssetServer`] rooted at the workspace `assets/`), so the editor's actual `Load` pass resolves
//! the shipped theme + content registries and its real `Editing` scene inserts the PREFAB model +
//! preview machinery — not a copy.
//!
//! Per `verification.md` Rule 3, this asserts the STATE-MACHINE + MODEL contract the egui DRAW +
//! the offscreen RENDER build on (the DRAW + render are Screenshot-QA-covered — the egui closure
//! never runs headlessly without a primary egui context, and the headless harness has no render
//! device):
//!
//! - the preview render-target resource ([`PreviewTarget`]) + the owned pan target ([`PreviewPan`])
//!   are inserted in `Editing` (the C4.3 / C4.8 state-scoped lifecycle),
//! - the change-driven redraw spawns preview tile [`Sprite`]s once the theme + registries resolve
//!   (proving the tile pipeline is wired — a non-empty viewport, not a black one — C4.3 / C4.12),
//! - a paint through the SHARED [`apply_placement`] predicate (the SAME one the viewport click
//!   runs — C4.4 / C4.10) mutates the [`EditorMap`], and the redraw picks it up.
//!
//! `assert!` + `let … else` keep the test panic-free per the workspace lints (no `unwrap` /
//! `expect` / `panic!`).

mod full_view;
mod harness;
mod preview;
mod storeys;
