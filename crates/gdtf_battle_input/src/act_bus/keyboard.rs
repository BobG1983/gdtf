//! Keyboard systems that push act intents.

use bevy::{input_focus::InputFocus, prelude::*};

use crate::{
    ActIntent, Keybinds, PendingActIntent, SelectedShooter,
    focus_bridge::{PanelNavOrder, focused_panel_button},
};

/// Push level-up / level-down intents from bound keys.
pub fn level_keys(
    keys: Res<ButtonInput<KeyCode>>,
    binds: Res<Keybinds>,
    mut pending: ResMut<PendingActIntent>,
) {
    if keys.just_pressed(binds.level_up()) {
        pending.push(ActIntent::LevelUp);
    }
    if keys.just_pressed(binds.level_down()) {
        pending.push(ActIntent::LevelDown);
    }
}

/// Push toggle-full-view intent from the bound key.
pub fn full_view_key(
    keys: Res<ButtonInput<KeyCode>>,
    binds: Res<Keybinds>,
    mut pending: ResMut<PendingActIntent>,
) {
    if keys.just_pressed(binds.toggle_full_view()) {
        pending.push(ActIntent::ToggleFullView);
    }
}

/// Clear selection when Escape is pressed and no panel button has focus.
pub fn select_clear_key(
    keys: Res<ButtonInput<KeyCode>>,
    binds: Res<Keybinds>,
    focus: Option<Res<InputFocus>>,
    panels: Query<(), With<PanelNavOrder>>,
    mut pending: ResMut<PendingActIntent>,
) {
    if focused_panel_button(focus.as_deref(), &panels).is_some() {
        return;
    }
    if keys.just_pressed(binds.select_clear()) {
        pending.push(ActIntent::SelectionClear);
    }
}

/// Stance cycle, aim toggle, and facing cycle for the selected shooter.
pub fn posture_keys(
    keys: Res<ButtonInput<KeyCode>>,
    binds: Res<Keybinds>,
    selected: Res<SelectedShooter>,
    mut pending: ResMut<PendingActIntent>,
) {
    if selected.is_none() {
        return;
    }
    if keys.just_pressed(binds.stance_cycle()) {
        pending.push(ActIntent::StanceCycle);
    }
    if keys.just_pressed(binds.aim_toggle()) {
        pending.push(ActIntent::AimToggle);
    }
    if keys.just_pressed(binds.facing_cycle()) {
        pending.push(ActIntent::FacingCycle);
    }
}

/// Cycle selection next/prev; Shift+Tab reverses direction.
pub fn cycle_selection_keys(
    keys: Res<ButtonInput<KeyCode>>,
    binds: Res<Keybinds>,
    focus: Option<Res<InputFocus>>,
    panels: Query<(), With<PanelNavOrder>>,
    mut pending: ResMut<PendingActIntent>,
) {
    if focused_panel_button(focus.as_deref(), &panels).is_some() {
        return;
    }
    if keys.just_pressed(binds.select_next()) {
        let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
        pending.push(if shift {
            ActIntent::SelectPrev
        } else {
            ActIntent::SelectNext
        });
    }
}
