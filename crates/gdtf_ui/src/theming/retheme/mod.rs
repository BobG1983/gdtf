//! The live-retheme seam: re-derive [`GdtfTheme`](crate::theme::GdtfTheme) when
//! the theme asset changes.
//!
//! GTW-137 makes the data-driven theme **hot-reloadable** in memory. When the
//! loose `assets/core_tuning/ui_theme.tuning.ron` asset is modified — by the OS file-watcher
//! in dev (GTW-138), or by a test injecting the message — Bevy emits an
//! [`AssetEvent::Modified`](bevy::asset::AssetEvent::Modified) for the theme
//! `RonAsset`. The GTW-564 GENERIC hot-RON redrive
//! ([`redrive_hot_ron_resource`](gdtf_assets::redrive_hot_ron_resource)
//! `::<GdtfThemeSpec, GdtfTheme>`, configured by [`theme_hot_ron_chain`] with
//! the [`resolve_theme_spec`] map hook) reacts to that message, re-derives the
//! resolved [`GdtfTheme`](crate::theme::GdtfTheme) from the **updated**
//! in-memory spec — re-resolving fonts through the
//! [`AssetServer`](bevy::asset::AssetServer) — and overwrites the
//! [`GdtfTheme`](crate::theme::GdtfTheme) resource in place.
//!
//! [`GdtfThemeSpec`]: crate::theme::GdtfThemeSpec
//!
//! ## Why this is all it does (the cadence lives elsewhere)
//!
//! This system does **not** re-paint widgets itself. Overwriting the
//! [`GdtfTheme`](crate::theme::GdtfTheme) resource through [`ResMut`](bevy::prelude::ResMut)
//! marks it *changed*, and [`apply_theme`](crate::themed::apply_theme) — made
//! change-driven by GTW-144 — then repaints every
//! [`Themed`](crate::themed::Themed) entity the same frame. For that to happen in
//! one frame, [`UiPlugin`](crate::UiPlugin) orders this system **before** the
//! [`UiSystems::ApplyTheme`](crate::themed::UiSystems::ApplyTheme) set
//! (bevy-traps rule 3).
//!
//! ## Engine-event driven, not file-watcher driven
//!
//! This is the deterministic, headless-testable LOGIC: it reacts to an
//! [`AssetEvent`](bevy::asset::AssetEvent) **however it arrives** — a real
//! file-watcher in dev or an injected message in a test. The OS file-watcher
//! wiring (the `AssetPlugin` watch source) is GTW-138 and is *not* part of this
//! module.

mod system;
#[cfg(test)]
mod test;

pub use system::{resolve_theme_spec, theme_hot_ron_chain};
