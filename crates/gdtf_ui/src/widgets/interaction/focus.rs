//! Sync pointer hover onto keyboard/gamepad focus.

use bevy::{
    input_focus::{FocusCause, InputFocus},
    prelude::*,
    ui::{Interaction, widget::Button},
};

use crate::widgets::core::DisabledButton;

type HoverableButton = (Changed<Interaction>, With<Button>, Without<DisabledButton>);

/// When a button is hovered, set input focus to it.
pub fn sync_hover_to_focus(
    mut focus: ResMut<InputFocus>,
    buttons: Query<(Entity, &Interaction), HoverableButton>,
) {
    for (entity, interaction) in &buttons {
        if *interaction == Interaction::Hovered {
            focus.set(entity, FocusCause::Navigated);
        }
    }
}
