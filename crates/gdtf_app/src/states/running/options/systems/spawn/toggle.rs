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
