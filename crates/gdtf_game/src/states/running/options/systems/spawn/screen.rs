//! Options screen spawn via `bsn!` with first-party widgets.
use bevy::{
    input_focus::directional_navigation::DirectionalNavigationMap,
    math::CompassOctant,
    prelude::*,
    scene::{CommandsSceneExt, bsn, template_value},
    ui::Val,
};
use gdtf_ui::{
    ButtonLabel,
    focus_nav::set_initial_focus,
    spawn_button, spawn_panel,
    theme::GdtfTheme,
    themed::{ThemeRole, Themed},
};

use super::{
    layout::OptionsGapVh,
    row::{SettingCaption, SettingRowSpec, SettingValueText, spawn_setting_row},
};
use crate::states::{
    RunningState,
    running::options::{
        components::{
            ContinueButton, OptionsScreenRoot, OptionsTitle, SoundToggle, SoundToggleKnob,
            SoundValueLabel,
        },
        settings::{GameSettings, sound_value_text},
    },
};

pub(in crate::states::running::options) fn spawn_options_screen(
    mut commands: Commands,
    theme: Option<Res<GdtfTheme>>,
    settings: Res<GameSettings>,
    mut nav_map: ResMut<DirectionalNavigationMap>,
) {
    let Some(theme) = theme else {
        return;
    };

    let root_node = Node {
        width: Val::Percent(100.0),
        height: Val::Percent(100.0),
        flex_direction: FlexDirection::Column,
        align_items: AlignItems::Center,
        justify_content: JustifyContent::Center,
        row_gap: Val::Vh(*OptionsGapVh::SCREEN),
        ..default()
    };
    let root = commands
        .spawn_scene((
            bsn! { Themed::new(ThemeRole::Background) OptionsScreenRoot },
            template_value(root_node),
            template_value(DespawnOnExit(RunningState::Options)),
        ))
        .id();

    let title = commands
        .spawn_scene((
            bsn! {
                Themed::new(ThemeRole::Title)
                Text::new("OPTIONS")
                OptionsTitle
            },
            template_value(TextLayout::justify(Justify::Center)),
            template_value(DespawnOnExit(RunningState::Options)),
        ))
        .id();

    let panel = spawn_panel(&mut commands, &theme);
    commands.entity(panel).insert((
        DespawnOnExit(RunningState::Options),
        Node {
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Stretch,
            row_gap: Val::Vh(*OptionsGapVh::SCREEN),
            ..default()
        },
    ));

    let sound = settings.sound;
    let sound_row = spawn_setting_row(
        &mut commands,
        &theme,
        SettingRowSpec {
            caption:       SettingCaption::new("Sound"),
            value:         sound,
            value_text:    SettingValueText::new(sound_value_text(sound)),
            toggle_marker: SoundToggle,
            knob_marker:   SoundToggleKnob,
            value_marker:  SoundValueLabel,
        },
    );

    let mut rows = vec![sound_row.row];
    let mut focusables = vec![sound_row.toggle];
    #[cfg(feature = "dev_tools")]
    {
        let stepper_row = super::stepper_row::spawn_stepper_row(&mut commands, &theme, *settings);
        rows.push(stepper_row.row);
        focusables.push(stepper_row.toggle);
    }

    let continue_button = spawn_button(
        &mut commands,
        &theme,
        ButtonLabel::new("Continue"),
        (ContinueButton, DespawnOnExit(RunningState::Options)),
    );
    commands.entity(continue_button).insert(Node {
        width: Val::Percent(100.0),
        ..default()
    });
    focusables.push(continue_button);
    rows.push(continue_button);

    commands.entity(root).add_children(&[title, panel]);
    commands.entity(panel).add_children(&rows);

    nav_map.add_edges(&focusables, CompassOctant::South);

    set_initial_focus(&mut commands, sound_row.toggle);
}

pub(in crate::states::running::options) fn clear_options_nav_map(
    mut nav_map: ResMut<DirectionalNavigationMap>,
) {
    nav_map.clear();
}
