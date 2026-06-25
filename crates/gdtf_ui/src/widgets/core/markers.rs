//! The widget markers and the button caption newtype.

use bevy::prelude::*;

/// Marker tagging a button that is currently disabled.
///
/// A disabled button is **painted** ([`paint_disabled_buttons`](super::paint_disabled_buttons)
/// writes the button sub-theme's explicit [`DisabledColor`](crate::theme::DisabledColor)
/// fill over its base) and is **skipped** by the interaction layer (the interaction system
/// queries `Without<DisabledButton>`, so a disabled button never hover/press-swaps).
/// It remains [`Themed`](crate::themed::Themed): the central base-look pass still
/// re-paints it, and the disabled fill is composed on top of that fresh base.
///
/// A unit marker — it carries no data; presence alone is the signal
/// (no-bare-types rule).
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct DisabledButton;

/// Marker tagging a button whose toggle is currently **ON** / active (GTW-253).
///
/// An active button is **painted** ([`paint_active_buttons`](super::paint_active_buttons)
/// writes the button sub-theme's explicit [`ActiveColor`](crate::theme::ActiveColor) fill
/// over its base), so a toggled-on control reads as persistently engaged (e.g. the Aim
/// button while the selected ganger is aiming). It remains
/// [`Themed`](crate::themed::Themed): the central base-look pass still re-paints it,
/// and the active fill is composed on top of that fresh base.
///
/// An active button stays fully INTERACTIVE — you click an active toggle to turn it OFF
/// (its `Interaction` is still driven, and the GTW-122 mouse-action layer still reads its
/// presses). What changed in GTW-266 is purely its PAINT: active is now **sticky** over
/// hover/press feedback. The interaction-feedback system
/// ([`theme_interaction`](crate::theme_interaction)) EXCLUDES `ActiveButton`
/// (`Without<ActiveButton>`), so an active button's color comes ONLY from
/// [`paint_active_buttons`](super::paint_active_buttons) — no hover/press swap clobbers the
/// active fill (that one-frame flicker was the user's GTW-266 complaint, and the toggle
/// panels in the Mode/Stance build need the current selection to read as steadily engaged).
/// When a button is BOTH [`DisabledButton`] and [`ActiveButton`], DISABLED wins:
/// [`paint_active_buttons`](super::paint_active_buttons) skips disabled buttons
/// (`Without<DisabledButton>`), so a disabled+active button keeps the disabled fill.
///
/// A unit marker — it carries no data; presence alone is the signal
/// (no-bare-types rule).
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct ActiveButton;

/// The visible caption of a [`spawn_button`](super::spawn_button) widget.
///
/// A named newtype over the caption string rather than a bare `String`
/// (no-bare-types rule): a button's label is a domain value, not arbitrary text.
#[derive(Deref, Clone, PartialEq, Eq, Debug)]
pub struct ButtonLabel(String);

impl ButtonLabel {
    /// Wraps a caption into a [`ButtonLabel`].
    #[must_use]
    pub fn new(label: impl Into<String>) -> Self {
        Self(label.into())
    }

    /// Consumes the label, yielding its inner caption string for
    /// [`Text::new`](bevy::prelude::Text::new).
    #[must_use]
    pub fn into_inner(self) -> String {
        self.0
    }
}
