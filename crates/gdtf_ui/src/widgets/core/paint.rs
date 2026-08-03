use bevy::{prelude::*, ui::BackgroundColor};

use super::markers::{ActiveButton, DisabledButton};
use crate::theme::GdtfTheme;

pub fn paint_disabled_buttons(
    theme: Option<Res<GdtfTheme>>,
    mut disabled: Query<&mut BackgroundColor, With<DisabledButton>>,
) {
    let Some(theme) = theme else {
        return;
    };

    let fill = *theme.button.disabled;
    for mut background in &mut disabled {
        background.0 = fill;
    }
}

pub fn paint_active_buttons(
    theme: Option<Res<GdtfTheme>>,
    mut active: Query<&mut BackgroundColor, (With<ActiveButton>, Without<DisabledButton>)>,
) {
    let Some(theme) = theme else {
        return;
    };

    let fill = *theme.button.active;
    for mut background in &mut active {
        background.0 = fill;
    }
}

#[cfg(test)]
mod test;
