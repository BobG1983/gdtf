use bevy::{input_focus::InputFocus, prelude::*};
use gdtf_battle_input::{BoundKey, Keybinds, PanelNavOrder, focused_panel_button};
use gdtf_ui::focus_nav::{FocusCancelled, NavDirection, NavigateRequest};

pub(in crate::states::running::game::battlescape) fn bridge_panel_focus_nav(
    keys: Option<Res<ButtonInput<KeyCode>>>,
    binds: Res<Keybinds>,
    focus: Option<Res<InputFocus>>,
    panels: Query<(), With<PanelNavOrder>>,
    mut navigate: MessageWriter<NavigateRequest>,
    mut cancel: MessageWriter<FocusCancelled>,
) {
    let Some(keys) = keys else {
        return;
    };
    if focused_panel_button(focus.as_deref(), &panels).is_none() {
        return;
    }

    if keys.just_pressed(binds.select_next()) {
        let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
        navigate.write(NavigateRequest::new(if shift {
            NavDirection::WEST
        } else {
            NavDirection::EAST
        }));
    }
    if keys.just_pressed(BoundKey::KeyArrowLeft.key_code()) {
        navigate.write(NavigateRequest::new(NavDirection::WEST));
    }
    if keys.just_pressed(BoundKey::KeyArrowRight.key_code()) {
        navigate.write(NavigateRequest::new(NavDirection::EAST));
    }
    if keys.just_pressed(binds.select_clear()) {
        cancel.write(FocusCancelled);
    }
}

pub(in crate::states::running::game::battlescape) fn apply_focus_cancel(
    mut cancelled: MessageReader<FocusCancelled>,
    focus: Option<ResMut<InputFocus>>,
) {
    if cancelled.read().count() == 0 {
        return;
    }
    if let Some(mut focus) = focus {
        focus.clear();
    }
}
