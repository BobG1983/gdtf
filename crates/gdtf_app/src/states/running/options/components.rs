//! Marker components for the Options screen's entities (GTW-637).
//!
//! Each entity the screen spawns carries a unit-struct marker naming its role, so
//! the action / adapter / theming systems (and the headless tests) find a specific
//! node by meaning rather than spawn order or label text — the same
//! identity-by-named-type convention the menu markers use (no-bare-types rule).
//!
//! The markers the external integration tests name go through
//! [`crate::support_item!`] (the GTW-145 test-only visibility flip): `pub` under
//! the `test-support` feature so `crate::test_support` can re-export them, and
//! `pub(crate)` otherwise so they stay internal and `unreachable_pub`-clean in the
//! production binary.

use bevy::prelude::*;

crate::support_item! {
    /// Marks the Options screen's full-screen backdrop ROOT node
    /// ([`Themed(ThemeRole::Background)`](gdtf_ui::themed::Themed)), the entity the
    /// rest of the screen parents under and the tests assert exists.
    ///
    /// A unit marker: presence on an entity is the whole signal (no-bare-types rule).
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct OptionsScreenRoot;
}

crate::support_item! {
    /// Marks the Options screen's title text ("OPTIONS"), which carries the
    /// [`Themed(ThemeRole::Title)`](gdtf_ui::themed::Themed) heading role.
    ///
    /// A unit marker: presence on an entity is the whole signal (no-bare-types rule).
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct OptionsTitle;
}

crate::support_item! {
    /// Marks the sound-toggle [`Checkbox`](bevy::ui_widgets::Checkbox) — the ONE
    /// first-party widget the pilot ships. The widget's native
    /// [`ValueChange<bool>`](bevy::ui_widgets::ValueChange) observer reads this marker
    /// to map an activation to the typed `SoundSettingChanged` intent (a scene-private
    /// message), and the theming pass reads it to paint the toggle's visual from the
    /// theme.
    ///
    /// A unit marker: presence on an entity is the whole signal (no-bare-types rule).
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct SoundToggle;
}

/// Marks the KNOB child of the sound-toggle [`Checkbox`](bevy::ui_widgets::Checkbox) —
/// the pip inside the pill that the theming pass repaints and re-justifies (start for
/// off, end for on) when the setting or the theme changes.
///
/// Screen-internal (no external test names it), so a plain restricted marker rather
/// than a `support_item!` test-visible one. A unit marker: presence on an entity is the
/// whole signal (no-bare-types rule).
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub(in crate::states::running::options) struct SoundToggleKnob;

crate::support_item! {
    /// Marks the text node that reads out the sound setting's current value
    /// ("On" / "Off"), mutated in place by the adapter when the toggle flips.
    ///
    /// A unit marker: presence on an entity is the whole signal (no-bare-types rule).
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct SoundValueLabel;
}

crate::support_item! {
    /// Marks the **Continue** button — activating it leaves the Options screen for
    /// [`RunningState::Game`](crate::states::RunningState::Game) (the screen is a
    /// real interactive stop now, not an auto-advance stub).
    ///
    /// A unit marker: presence on an entity is the whole signal (no-bare-types rule).
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct ContinueButton;
}
