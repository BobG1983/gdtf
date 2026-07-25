//! Theme integration + visual for the Options screen's first-party toggle widgets — the
//! sound-toggle [`Checkbox`](bevy::ui_widgets::Checkbox) (GTW-637, THEMING clause) and,
//! in a `dev_tools` build, the procgen-stepper toggle (GTW-868).
//!
//! `bevy_ui_widgets::Checkbox` is a HEADLESS widget: it carries no visual and no
//! painting of its own (it is deliberately render-agnostic so an app styles it however
//! it likes). So the ENTIRE look of a toggle — a pill track with a sliding knob — is the
//! screen's to build (in [`spawn`](super::spawn)) and to paint from the RON theme, and to
//! REPAINT in place on both a setting flip and a hot-reload. This module is the honest
//! measure of that adapter tax:
//!
//! - [`toggle_colors`] derives a toggle's off / on / knob / border colors from the
//!   button sub-theme of the live [`GdtfTheme`](gdtf_ui::theme::GdtfTheme). The spawn
//!   path calls it for the initial paint.
//! - [`paint_sound_toggle`] (and, under `dev_tools`,
//!   [`paint_stepper_toggle`](super::paint_stepper_toggle)) re-derives those colors and
//!   MUTATES the toggle's track [`BackgroundColor`], its track
//!   [`BorderColor`](bevy::ui::BorderColor) pill outline, its track [`Node`] justify
//!   (which end the knob sits at), and its knob's [`BackgroundColor`] in place — no
//!   despawn/respawn (the project's mutate-in-place UI convention). It runs when EITHER
//!   the setting changes (the flip) or the theme changes (the hot-reload), so the same
//!   code repaints the toggle for a toggle and for a live theme swap, staying in step
//!   with every [`Themed`](gdtf_ui::themed::Themed) node.
//!
//! The colors, the knob justify, and the repaint body are shared across BOTH toggles
//! through [`ToggleValue`] — a tiny trait each setting newtype implements — so the
//! dev-only toggle is painted by the same code as the player-facing one rather than a
//! parallel copy.

use bevy::{
    prelude::*,
    ui::{BackgroundColor, BorderColor as UiBorderColor},
};
use gdtf_ui::theme::GdtfTheme;

#[cfg(feature = "dev_tools")]
use crate::states::running::options::settings::ProcgenStepperEnabled;
use crate::states::running::options::{
    components::{SoundToggle, SoundToggleKnob},
    settings::{GameSettings, SoundEnabled},
};

/// A settings value a pill toggle can render: it is either on or off.
///
/// Implemented by each setting's own newtype (never by a bare `bool`), so the shared
/// paint helpers below take a TYPED value rather than an opaque flag while still serving
/// every toggle on the screen (no-bare-types rule).
pub(in crate::states::running::options) trait ToggleValue:
    Copy
{
    /// Whether this setting is currently on.
    fn is_on(self) -> bool;
}

impl ToggleValue for SoundEnabled {
    fn is_on(self) -> bool {
        Self::is_on(self)
    }
}

#[cfg(feature = "dev_tools")]
impl ToggleValue for ProcgenStepperEnabled {
    fn is_on(self) -> bool {
        Self::is_on(self)
    }
}

/// The off-color / on-color pair a toggle's TRACK shows, plus the knob and border colors.
///
/// A small typed bundle (not four bare [`Color`](bevy::prelude::Color) parameters
/// threaded around, no-bare-types rule) derived from the theme by [`toggle_colors`] and
/// read by both the spawn paint and the repaint systems.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(in crate::states::running::options) struct ToggleColors {
    /// Track color while OFF.
    off:    Color,
    /// Track color while ON.
    on:     Color,
    /// The knob fill color (constant across the flip).
    knob:   Color,
    /// The track's pill-outline border color (constant across the flip) — its opaque
    /// outline is what keeps the OFF track visible against the panel (GTW-800b).
    border: Color,
}

impl ToggleColors {
    /// The track color for the given on/off state.
    pub(in crate::states::running::options) fn track<V: ToggleValue>(&self, value: V) -> Color {
        if value.is_on() { self.on } else { self.off }
    }

    /// The knob fill color (constant across the flip).
    pub(in crate::states::running::options) const fn knob(&self) -> Color {
        self.knob
    }

    /// The track's border (pill-outline) color (constant across the flip).
    pub(in crate::states::running::options) const fn border(&self) -> Color {
        self.border
    }
}

/// Derives a toggle's [`ToggleColors`] from the button sub-theme of `theme`.
///
/// OFF reads the muted disabled fill, ON the engaged active fill (GTW-253), the knob
/// the button caption color, and the pill outline the button border color — all
/// RON-driven, no literals — so a toggle reads against the panel exactly as the
/// themed buttons do and re-derives identically on a hot-reload. The opaque border is
/// what keeps the OFF track (a near-black, semi-transparent fill) distinguishable from
/// the panel (GTW-800b).
pub(in crate::states::running::options) fn toggle_colors(theme: &GdtfTheme) -> ToggleColors {
    ToggleColors {
        off:    *theme.button.disabled,
        on:     *theme.button.active,
        knob:   *theme.button.text_color,
        border: *theme.button.border_color,
    }
}

/// Which end of the track the knob sits at for the current setting.
///
/// OFF pins the knob to the START of the (row) track, ON to the END, mirroring the
/// hand-rolled switch's slide.
pub(in crate::states::running::options) fn knob_justify<V: ToggleValue>(
    value: V,
) -> JustifyContent {
    if value.is_on() {
        JustifyContent::FlexEnd
    } else {
        JustifyContent::FlexStart
    }
}

/// A toggle's mutable paint data: its track fill, its track pill-outline
/// [`BorderColor`](bevy::ui::BorderColor), its track [`Node`] (for the knob justify),
/// and its knob children. Aliased to keep the paint systems' signatures legible
/// (clippy `type_complexity`).
pub(in crate::states::running::options) type ToggleVisuals = (
    &'static mut BackgroundColor,
    &'static mut UiBorderColor,
    &'static mut Node,
    &'static Children,
);

/// Repaints one pill toggle's track + knob in place from `colors` and its current value.
///
/// Shared by every toggle on the screen: the caller supplies the marker-filtered queries
/// and the typed setting value, this mutates the track fill, the pill outline, the knob
/// justify, and the knob fill — never despawning or respawning anything.
pub(in crate::states::running::options) fn repaint_toggle<
    V: ToggleValue,
    T: Component,
    K: Component,
>(
    colors: ToggleColors,
    value: V,
    toggles: &mut Query<ToggleVisuals, With<T>>,
    knobs: &mut Query<&mut BackgroundColor, (With<K>, Without<T>)>,
) {
    for (mut track, mut border, mut node, children) in toggles {
        track.0 = colors.track(value);
        *border = UiBorderColor::all(colors.border());
        node.justify_content = knob_justify(value);
        for child in children {
            if let Ok(mut knob) = knobs.get_mut(*child) {
                knob.0 = colors.knob();
            }
        }
    }
}

/// Repaints the sound-toggle [`Checkbox`](bevy::ui_widgets::Checkbox) from the current
/// theme + setting, MUTATING its painted fills and knob position in place.
///
/// Runs when either [`GameSettings`] or [`GdtfTheme`](gdtf_ui::theme::GdtfTheme)
/// changed (the scene plugin's `run_if`): a setting flip repaints the track to the new
/// on/off color and slides the knob, and a `resource_changed::<GdtfTheme>` hot-reload
/// re-derives the palette and repaints — the same body serves both. Guarded
/// `Option<Res<GdtfTheme>>` so it is inert before the theme is populated (bevy-traps
/// rule 1).
pub(in crate::states::running::options) fn paint_sound_toggle(
    theme: Option<Res<GdtfTheme>>,
    settings: Res<GameSettings>,
    mut toggles: Query<ToggleVisuals, With<SoundToggle>>,
    mut knobs: Query<&mut BackgroundColor, (With<SoundToggleKnob>, Without<SoundToggle>)>,
) {
    let Some(theme) = theme else {
        return;
    };
    let colors = toggle_colors(&theme);
    repaint_toggle(colors, settings.sound, &mut toggles, &mut knobs);
}
