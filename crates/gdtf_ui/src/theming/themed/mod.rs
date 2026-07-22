//! The hot-reload paint pass: the [`Themed`] marker and the central [`apply_theme`]
//! system that paints theme-derived visuals onto themed entities.
//!
//! An entity opts into centralized theming by carrying a [`Themed`] component,
//! whose [`ThemeRole`] declares *what kind* of widget it is. On every run
//! [`apply_theme`] reads the **current** [`GdtfTheme`](crate::theme::GdtfTheme)
//! resource and writes that entity's base look from the matching **sub-theme**
//! (GTW-149):
//!
//! - [`ThemeRole::Background`] (a [`Node`](bevy::ui::Node)): the full-screen
//!   backdrop fill from `background.color` — no border/radius/padding.
//! - [`ThemeRole::Panel`] (a [`Node`](bevy::ui::Node)): fill, border color, border
//!   width, corner radius, and content padding from `panel.*` (a box around UI).
//! - [`ThemeRole::Button`] (a [`Node`](bevy::ui::Node)): fill, border color,
//!   border width, corner radius, and content padding from `button.*` (the button
//!   box).
//! - [`ThemeRole::ButtonText`] (a [`Text`](bevy::prelude::Text)): text color, font
//!   face, and font size from the **button** sub-theme.
//! - [`ThemeRole::Title`] (a [`Text`](bevy::prelude::Text)): text color, font face,
//!   and font size from the **title** sub-theme.
//! - [`ThemeRole::Text`] (a [`Text`](bevy::prelude::Text)): text color, font face,
//!   and font size from the **text** sub-theme.
//!
//! Each text role draws its own `font_size_pt` from its sub-theme — there is no
//! scale-the-body-size title fudge anymore; the title simply has its own size.
//!
//! ## Live read, never a spawn snapshot
//!
//! [`apply_theme`] resolves its values from the [`GdtfTheme`](crate::theme::GdtfTheme) resource *every
//! run*, not at spawn time. Re-running it after the resource is mutated re-paints
//! every [`Themed`] entity with the new palette — that is what the live
//! hot-reload hangs on.
//!
//! ## Boundary with interaction state
//!
//! [`apply_theme`] sets only the **base** look. Per-widget interaction feedback
//! (hover / press background swaps) composes *on top* of this base in a later
//! system, ordered after [`UiSystems::ApplyTheme`]; it is intentionally not part
//! of this system.
//!
//! ## Cadence and absence guard
//!
//! [`apply_theme`] is **change-driven** (GTW-144): [`UiPlugin`](crate::UiPlugin)
//! registers it to run only when the [`GdtfTheme`](crate::theme::GdtfTheme)
//! resource *changed* (the `Load` insert or the re-derive — repainting every
//! [`Themed`] entity, i.e. the retheme) **or** when a new [`Themed`] entity was
//! added this frame (so a freshly-spawned widget still gets its base look). It
//! does **not** run on steady-state frames, so it never clobbers the GTW-118
//! hover/press feedback. The `resource_exists::<GdtfTheme>` part of that run
//! condition also keeps it inert before `AppState::Load` populates the resource
//! (bevy-traps rule 1) — it never panics on an absent theme.

mod role;
mod system;
#[cfg(test)]
mod test;

pub use role::{ThemeRole, Themed, UiSystems};
pub use system::{any_themed_added, apply_theme};
