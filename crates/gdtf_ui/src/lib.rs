//! Hand-rolled `bevy_ui` layer for GDTF.
//!
//! This crate is the seam the menu / HUD work hangs on. It owns no combat rules
//! (those live in `gdtf_battle_sim`) and deliberately depends on **bevy only**,
//! so `gdtf_app` can depend on it without forming a dependency cycle.
//!
//! [`UiPlugin`] is the single registration seam: today it installs the
//! [`focus_nav`] sub-plugin ([`FocusNavPlugin`](focus_nav::FocusNavPlugin)) and
//! nothing else. Later tickets attach further UI systems, resources, and assets
//! to [`UiPlugin::build`].
//!
//! The [`focus_nav`] module wires Bevy's `input_focus` framework and bridges
//! keyboard + gamepad input onto directional focus navigation; see its docs for
//! the activation-message decision.
//!
//! The data-driven [`theme`] module defines the on-disk theme schema, the runtime
//! [`GdtfTheme`](theme::GdtfTheme) resource, and the pure spec-to-resource
//! resolution; population of that resource lands with later tickets.
//!
//! The [`themed`] module owns the [`Themed`](themed::Themed) marker and the
//! central [`apply_theme`](themed::apply_theme) system — the hot-reload seam that
//! paints theme-derived visuals onto themed entities from the live
//! [`GdtfTheme`](theme::GdtfTheme).
//!
//! The [`retheme`] module owns the live-reapply logic
//! ([`redrive_theme_on_asset_event`](retheme::redrive_theme_on_asset_event)): on
//! an [`AssetEvent`](bevy::asset::AssetEvent)`::Modified` for the active theme
//! asset it re-derives [`GdtfTheme`](theme::GdtfTheme) in place, and the
//! change-driven [`apply_theme`](themed::apply_theme) repaints every
//! [`Themed`](themed::Themed) entity the same frame — no restart (GTW-137).
//!
//! The [`widgets`] module owns the reusable spawn helpers
//! ([`spawn_panel`](widgets::spawn_panel) / [`spawn_button`](widgets::spawn_button)),
//! the [`DisabledButton`](widgets::DisabledButton) / [`ActiveButton`](widgets::ActiveButton)
//! markers, and their paint passes
//! ([`paint_disabled_buttons`](widgets::paint_disabled_buttons) /
//! [`paint_active_buttons`](widgets::paint_active_buttons)); the [`interaction`]
//! module owns the theme-derived hover/press feedback system. All compose *on top
//! of* [`apply_theme`](themed::apply_theme)'s base look, ordered after it.
//!
//! The [`UiPlugin`] registration seam itself lives in the private `plugin`
//! submodule and is re-exported here unchanged.

pub mod focus_nav;
pub mod interaction;
pub mod retheme;
pub mod theme;
pub mod themed;
pub mod widgets;

mod plugin;

pub use interaction::{sync_hover_to_focus, theme_interaction};
pub use plugin::UiPlugin;
pub use retheme::redrive_theme_on_asset_event;
pub use themed::any_themed_added;
pub use widgets::{
    ActiveButton, ButtonLabel, DisabledButton, paint_active_buttons, paint_disabled_buttons,
    spawn_button, spawn_panel,
};
