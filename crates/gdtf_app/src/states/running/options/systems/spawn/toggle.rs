//! The shared pill-toggle builder every Options setting row uses.
//!
//! `bevy_ui_widgets::Checkbox` is headless, so the whole visual is the screen's to build:
//! a rounded track [`Node`] (colored from the theme for the current on/off state, with an
//! opaque themed pill outline so it stays visible against the panel when OFF — GTW-800b)
//! with one knob child (justified to the on/off end). Building it ONCE here means the
//! GTW-868 dev-only stepper toggle is the sound toggle's twin by construction rather than
//! a copied visual that can drift.

use bevy::{
    prelude::*,
    ui::{BorderColor as UiBorderColor, BorderRadius, Checked, UiRect, Val},
    ui_widgets::Checkbox,
};
use gdtf_ui::theme::GdtfTheme;

use super::layout::{KNOB_DIAMETER_VW, TOGGLE_LONG_VW, TOGGLE_PAD_VW, TOGGLE_SHORT_VW};
use crate::states::{
    RunningState,
    running::options::systems::theming::{ToggleValue, knob_justify, toggle_colors},
};

/// Spawns one setting's [`Checkbox`](bevy::ui_widgets::Checkbox) as a themed pill toggle
/// and returns the checkbox [`Entity`].
///
/// `marker` is the setting's own toggle marker bundle and `knob_marker` its knob marker —
/// the two identities the activation observer and the theming pass filter on. The checkbox
/// is seeded from `value` (the first-party [`Checked`](bevy::ui::Checked) component IS the
/// checkbox's state, kept in step by the `checkbox_self_update` observer the scene plugin
/// registers), and the whole tree is marked [`DespawnOnExit(RunningState::Options)`].
pub(super) fn spawn_toggle<V: ToggleValue, M: Bundle, K: Bundle>(
    commands: &mut Commands,
    theme: &GdtfTheme,
    value: V,
    marker: M,
    knob_marker: K,
) -> Entity {
    let colors = toggle_colors(theme);
    let track_node = Node {
        width: Val::Vw(TOGGLE_LONG_VW),
        height: Val::Vw(TOGGLE_SHORT_VW),
        flex_direction: FlexDirection::Row,
        padding: UiRect::all(Val::Vw(TOGGLE_PAD_VW)),
        align_items: AlignItems::Center,
        justify_content: knob_justify(value),
        // A visible pill outline (from the button sub-theme's border, opaque) so the
        // track reads against the panel in BOTH states — the OFF fill alone
        // (theme.button.disabled, near-black at 0.55 alpha) blends into the
        // semi-transparent panel and the pill was invisible when off (GTW-800b).
        border: UiRect::all(Val::Vw(*theme.button.border_width)),
        border_radius: BorderRadius::all(Val::Percent(50.0)),
        ..default()
    };
    let knob_node = Node {
        width: Val::Vw(KNOB_DIAMETER_VW),
        height: Val::Vw(KNOB_DIAMETER_VW),
        border_radius: BorderRadius::all(Val::Percent(50.0)),
        ..default()
    };
    let knob = commands
        .spawn((
            knob_marker,
            BackgroundColor(colors.knob()),
            knob_node,
            DespawnOnExit(RunningState::Options),
        ))
        .id();
    let toggle = commands
        .spawn((
            Checkbox,
            marker,
            BackgroundColor(colors.track(value)),
            UiBorderColor::all(colors.border()),
            track_node,
            DespawnOnExit(RunningState::Options),
        ))
        .id();
    if value.is_on() {
        commands.entity(toggle).insert(Checked);
    }
    commands.entity(toggle).add_children(&[knob]);
    toggle
}
