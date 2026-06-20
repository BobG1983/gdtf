//! The [`Themed`] marker, its [`ThemeRole`] vocabulary, and the
//! [`UiSystems`] ordering set.

use bevy::prelude::*;

/// The kind of themed widget an entity is, which selects *which* sub-theme
/// [`apply_theme`](super::apply_theme) paints onto it.
///
/// A named role rather than a bare boolean or marker pair: the set of widget
/// kinds the theme knows how to paint is a closed vocabulary, and a closed
/// vocabulary is an enum.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ThemeRole {
    /// The full-screen backdrop [`Node`](bevy::ui::Node):
    /// [`apply_theme`](super::apply_theme) sets its background fill from the
    /// **background** sub-theme (no border/radius/padding).
    Background,
    /// A panel-box [`Node`](bevy::ui::Node): [`apply_theme`](super::apply_theme)
    /// sets its fill, border color, border width, corner radius, and content
    /// padding from the **panel** sub-theme.
    Panel,
    /// A button-box [`Node`](bevy::ui::Node): [`apply_theme`](super::apply_theme)
    /// sets its fill, border color, border width, corner radius, and content
    /// padding from the **button** sub-theme. Interaction feedback (GTW-118)
    /// composes hover/press on top.
    Button,
    /// A button caption [`Text`](bevy::prelude::Text):
    /// [`apply_theme`](super::apply_theme) sets its color, font face, and size
    /// from the **button** sub-theme.
    ButtonText,
    /// A heading / title [`Text`](bevy::prelude::Text):
    /// [`apply_theme`](super::apply_theme) sets its color, font face, and size
    /// from the **title** sub-theme.
    Title,
    /// A body-text [`Text`](bevy::prelude::Text):
    /// [`apply_theme`](super::apply_theme) sets its color, font face, and size
    /// from the **text** sub-theme.
    Text,
}

/// Marker opting an entity into centralized theming, tagged with its
/// [`ThemeRole`].
///
/// Attach `Themed(role)` to any UI entity whose look should be driven by the
/// live [`GdtfTheme`](crate::theme::GdtfTheme): [`apply_theme`](super::apply_theme)
/// then paints the role-appropriate visuals onto it every run, re-reading the
/// resource so a theme change re-themes it. Spawning code only declares the
/// *role*; it never copies theme values itself, which is what keeps the look in
/// one place and live.
#[derive(Component, Deref, Clone, Copy, PartialEq, Eq, Debug)]
pub struct Themed(ThemeRole);

impl Themed {
    /// Tag an entity with the [`ThemeRole`] `apply_theme` should paint onto it.
    #[must_use]
    pub const fn new(role: ThemeRole) -> Self {
        Self(role)
    }
}

/// Explicit system-ordering sets for the UI theming layer.
///
/// [`ApplyTheme`](UiSystems::ApplyTheme) names where
/// [`apply_theme`](super::apply_theme) runs so later systems can order
/// deterministically relative to it (bevy-traps rule 3) — in particular the
/// retheme trigger, and any interaction-feedback system (GTW-118) that must
/// compose its hover/press swap *after* the base look is laid down.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UiSystems {
    /// The set containing [`apply_theme`](super::apply_theme) — the base-look
    /// application pass.
    ApplyTheme,
}
