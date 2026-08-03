use bevy::prelude::*;
use gdtf_battle_input::{ActIntent, PendingActIntent, SelectedShooter};
use gdtf_battle_sim::ganger::Aiming;
use gdtf_ui::{
    Orientation, Switch, SwitchColors, SwitchOrientation, SwitchState, ToggleFlipped, spawn_switch,
    theme::GdtfTheme,
};

use crate::states::running::game::battlescape::action_bar::components::AimToggleButton;

pub(in crate::states::running::game::battlescape) fn spawn_aim_button(
    commands: &mut Commands,
    theme: &GdtfTheme,
) -> Entity {
    spawn_switch(
        commands,
        SwitchState::Off,
        aim_switch_colors(theme),
        Orientation::Horizontal,
        AimToggleButton,
    )
}

fn aim_switch_colors(theme: &GdtfTheme) -> SwitchColors {
    SwitchColors {
        off:  off_track_color(*theme.button.color),
        on:   *theme.button.active,
        knob: *theme.button.text_color,
    }
}

fn off_track_color(button: Color) -> Color {
        const LIGHTEN: f32 = 0.5;
    let srgba = button.to_srgba();
    Color::srgb(
        (1.0 - srgba.red).mul_add(LIGHTEN, srgba.red),
        (1.0 - srgba.green).mul_add(LIGHTEN, srgba.green),
        (1.0 - srgba.blue).mul_add(LIGHTEN, srgba.blue),
    )
}

pub(in crate::states::running::game::battlescape) fn aim_switch_flip_intent(
    mut flipped: MessageReader<ToggleFlipped>,
    mut pending: ResMut<PendingActIntent>,
    aim_switches: Query<(), With<AimToggleButton>>,
) {
    for event in flipped.read() {
        if aim_switches.get(event.switch).is_ok() {
            pending.push(ActIntent::AimToggle);
        }
    }
}

type AimSwitchData = (
    &'static mut SwitchState,
    &'static SwitchColors,
    &'static SwitchOrientation,
    &'static mut BackgroundColor,
    &'static mut Node,
);

type AimSwitchFilter = (With<AimToggleButton>, With<Switch>);

pub(in crate::states::running::game::battlescape) fn sync_aim_switch_state(
    selected: Res<SelectedShooter>,
    aiming: Query<&Aiming>,
    mut switches: Query<AimSwitchData, AimSwitchFilter>,
) {
    let is_aiming = (**selected)
        .and_then(|entity| aiming.get(entity).ok())
        .is_some_and(|aim| **aim);
    let want = if is_aiming {
        SwitchState::On
    } else {
        SwitchState::Off
    };

    for (mut state, colors, orientation, mut background, mut node) in &mut switches {
        if *state != want {
            *state = want;
            background.0 = colors.track(want);
            node.flex_direction = orientation.flex_direction();
            node.justify_content = knob_justify(want);
        }
    }
}

const fn knob_justify(state: SwitchState) -> JustifyContent {
    match state {
        SwitchState::Off => JustifyContent::FlexStart,
        SwitchState::On => JustifyContent::FlexEnd,
    }
}
