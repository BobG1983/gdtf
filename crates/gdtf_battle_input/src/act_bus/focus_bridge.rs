//! GTW-782: the panel-focus CONTEXT primitives — the [`PanelNavOrder`] marker every
//! focus-navigable battlescape HUD button wears, and the [`focused_panel_button`]
//! predicate the mutually-exclusive Tab / Escape modes gate on.
//!
//! ## Why these live on the input side (and the message bridge does not)
//!
//! The user's ruling makes Tab / Escape CONTEXT-GATED on whether a battlescape HUD panel
//! currently holds keyboard focus: with a panel focused, Tab drives panel focus-nav and
//! Escape backs out of it; with no panel focused, they keep their cycle-ganger /
//! clear-selection meaning. The gate ("a panel holds focus") is needed by the existing
//! [`cycle_selection_keys`](crate::keyboard::cycle_selection_keys) /
//! [`select_clear_key`](crate::keyboard::select_clear_key) handlers, which live in THIS
//! crate — so the predicate and the marker it reads live here too.
//!
//! Crucially this needs NO `gdtf_ui` dependency: keyboard focus is tracked by
//! [`InputFocus`], a **bevy** resource (`bevy::input_focus`), not a `gdtf_ui` type. So the
//! context primitives stay in `gdtf_battle_input` (which does not depend on `gdtf_ui` in
//! its normal graph — only as a dev-dependency), while the message-feeding half of the
//! bridge — the part that WRITES `gdtf_ui`'s `NavigateRequest` / `FocusCancelled` from the
//! typed [`Keybinds`](crate::Keybinds) model — lives on the app side (`gdtf_app`, which
//! depends on both `gdtf_ui` and this crate). That split is exactly the "message-fed
//! bridge on the input/app side" the GTW-782 dependency constraint calls for.

use bevy::{input_focus::InputFocus, prelude::*};

/// Marks a battlescape HUD button as a keyboard focus-navigation participant and carries
/// its stable position in the Tab traversal chain.
///
/// Presence identifies "a battlescape panel button that keyboard focus can land on" — the
/// signal [`focused_panel_button`] tests to decide the panel-focus context — and the
/// wrapped ordinal fixes the button's slot in the left-to-right Tab chain the app-side
/// topology rebuild wires into the [`DirectionalNavigationMap`](bevy::input_focus::directional_navigation::DirectionalNavigationMap).
/// A named newtype over the raw ordinal (no-bare-types); the inner is private, read
/// through the derived [`Deref`] and ordered through the derived [`Ord`].
///
/// `gdtf_app`'s battlescape panels attach it (the app depends on this crate); this crate
/// only reads it — so the "focusable panel button" contract stays a named type, never a
/// bare marker.
#[derive(Component, Deref, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct PanelNavOrder(u16);

impl PanelNavOrder {
    /// Wrap a raw Tab-chain ordinal (lower = earlier in the left-to-right traversal).
    #[must_use]
    pub const fn new(order: u16) -> Self {
        Self(order)
    }
}

/// The battlescape panel button that currently holds keyboard focus, or [`None`] when no
/// panel does — the shared mode predicate.
///
/// Returns the focused entity when [`InputFocus`] points at an entity carrying
/// [`PanelNavOrder`], else [`None`]. [`InputFocus`] is taken as an [`Option`] so a harness
/// without the focus framework (no `InputFocus` resource) reads as "no panel focus" rather
/// than being unreachable. This is the single definition of "a battlescape panel currently
/// holds input focus" that both the app-side bridge and the
/// [`cycle_selection_keys`](crate::keyboard::cycle_selection_keys) /
/// [`select_clear_key`](crate::keyboard::select_clear_key) context guards gate on.
#[must_use]
pub fn focused_panel_button(
    focus: Option<&InputFocus>,
    panels: &Query<(), With<PanelNavOrder>>,
) -> Option<Entity> {
    let focused = focus?.get()?;
    panels.contains(focused).then_some(focused)
}
