use bevy::{
    input_focus::directional_navigation::DirectionalNavigationMap,
    math::CompassOctant,
    prelude::*,
    scene::{CommandsSceneExt, bsn, template_value},
    ui::Val,
};
use gdtf_ui::{
    ButtonLabel, DisabledButton, MenuItem, MenuName, MenuScreen,
    focus_nav::set_initial_focus,
    spawn_button, spawn_panel,
    theme::GdtfTheme,
    themed::{ThemeRole, Themed},
};

use crate::states::{
    RunningState,
    running::menu::components::{
        BattlescapeButton, HiveScapeButton, MenuTitle, OptionsButton, QuitButton,
    },
};

const MAIN_MENU_ID: &str = "MainMenu";

#[derive(Deref, Clone, Copy, PartialEq, Debug)]
struct ColumnGapVh(f32);

impl ColumnGapVh {
    const MENU: Self = Self(1.38889);
}

pub(in crate::states::running::menu) fn spawn_menu(
    mut commands: Commands,
    theme: Option<Res<GdtfTheme>>,
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
        row_gap: Val::Vh(*ColumnGapVh::MENU),
        ..default()
    };
    let root = commands
        .spawn_scene((
            bsn! { Themed::new(ThemeRole::Background) },
            template_value(root_node),
            template_value(DespawnOnExit(RunningState::Menu)),
        ))
        .id();
    commands
        .entity(root)
        .insert(MenuScreen::new(MenuName::new(MAIN_MENU_ID.to_owned())));

    let title = commands
        .spawn_scene((
            bsn! {
                Themed::new(ThemeRole::Title)
                Text::new("GRIMDARK TURFWAR")
                MenuTitle
            },
            template_value(TextLayout::justify(Justify::Center)),
            template_value(DespawnOnExit(RunningState::Menu)),
        ))
        .id();

    let panel = spawn_panel(&mut commands, &theme);
    commands.entity(panel).insert((
        DespawnOnExit(RunningState::Menu),
        Node {
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Stretch,
            row_gap: Val::Vh(*ColumnGapVh::MENU),
            ..default()
        },
    ));

    let battlescape = spawn_button(
        &mut commands,
        &theme,
        ButtonLabel::new("Battlescape"),
        (
            BattlescapeButton,
            MenuItem,
            DespawnOnExit(RunningState::Menu),
        ),
    );
    let options = spawn_button(
        &mut commands,
        &theme,
        ButtonLabel::new("Options"),
        (OptionsButton, MenuItem, DespawnOnExit(RunningState::Menu)),
    );
    let hivescape = spawn_button(
        &mut commands,
        &theme,
        ButtonLabel::new("HiveScape"),
        (
            HiveScapeButton,
            MenuItem,
            DisabledButton,
            DespawnOnExit(RunningState::Menu),
        ),
    );
    let quit = spawn_button(
        &mut commands,
        &theme,
        ButtonLabel::new("Quit"),
        (QuitButton, MenuItem, DespawnOnExit(RunningState::Menu)),
    );

    for button in [battlescape, options, hivescape, quit] {
        commands.entity(button).insert(Node {
            width: Val::Percent(100.0),
            ..default()
        });
    }

    commands.entity(root).add_children(&[title, panel]);
    commands
        .entity(panel)
        .add_children(&[battlescape, options, hivescape, quit]);

    set_initial_focus(&mut commands, battlescape);

    nav_map.add_edges(&[battlescape, options, quit], CompassOctant::South);
}
