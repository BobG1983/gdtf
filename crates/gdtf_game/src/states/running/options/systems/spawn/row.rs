use bevy::{
    prelude::*,
    scene::{CommandsSceneExt, bsn, template_value},
    ui::Val,
};
use gdtf_ui::{
    theme::GdtfTheme,
    themed::{ThemeRole, Themed},
};

use super::{layout::SETTING_VALUE_MIN_WIDTH_VW, toggle::spawn_toggle};
use crate::states::{RunningState, running::options::systems::theming::ToggleValue};

#[derive(Deref, Clone, Copy, PartialEq, Eq, Debug)]
pub(super) struct SettingCaption(&'static str);

impl SettingCaption {
    pub(super) const fn new(caption: &'static str) -> Self {
        Self(caption)
    }
}

#[derive(Deref, Clone, Copy, PartialEq, Eq, Debug)]
pub(super) struct SettingValueText(&'static str);

impl SettingValueText {
    pub(super) const fn new(text: &'static str) -> Self {
        Self(text)
    }
}

pub(super) struct SettingRowSpec<V, M, K, L> {
    pub(super) caption:       SettingCaption,
    pub(super) value:         V,
    pub(super) value_text:    SettingValueText,
    pub(super) toggle_marker: M,
    pub(super) knob_marker:   K,
    pub(super) value_marker:  L,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) struct SettingRow {
    pub(super) row:    Entity,
    pub(super) toggle: Entity,
}

pub(super) fn spawn_setting_row<V: ToggleValue, M: Bundle, K: Bundle, L: Bundle>(
    commands: &mut Commands,
    theme: &GdtfTheme,
    spec: SettingRowSpec<V, M, K, L>,
) -> SettingRow {
    let row = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: Val::Vw(*super::layout::OptionsGapVh::SCREEN),
                ..default()
            },
            DespawnOnExit(RunningState::Options),
        ))
        .id();

    let caption = *spec.caption;
    let label = commands
        .spawn_scene((
            bsn! { Themed::new(ThemeRole::Text) Text::new(caption) },
            template_value(DespawnOnExit(RunningState::Options)),
        ))
        .id();

    let toggle = spawn_toggle(
        commands,
        theme,
        spec.value,
        spec.toggle_marker,
        spec.knob_marker,
    );

    let value_node = Node {
        min_width: Val::Vw(SETTING_VALUE_MIN_WIDTH_VW),
        ..default()
    };
    let value_text = *spec.value_text;
    let value = commands
        .spawn_scene((
            bsn! { Themed::new(ThemeRole::Text) Text::new(value_text) },
            template_value(value_node),
            template_value(DespawnOnExit(RunningState::Options)),
        ))
        .id();
    commands.entity(value).insert(spec.value_marker);

    commands.entity(row).add_children(&[label, toggle, value]);
    SettingRow { row, toggle }
}
