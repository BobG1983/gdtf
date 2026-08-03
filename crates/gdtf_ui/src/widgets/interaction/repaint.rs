use bevy::{
    prelude::*,
    ui::{BackgroundColor, BorderColor as UiBorderColor, Interaction, widget::Button},
};

use super::theme::interaction_fill;
use crate::{
    theme::GdtfTheme,
    widgets::core::{ActiveButton, DisabledButton, Segment, Switch},
};

type DeactivatedButton = (
    With<Button>,
    Without<DisabledButton>,
    Without<ActiveButton>,
    Without<Segment>,
);

type DeactivationVisuals = (
    &'static Interaction,
    &'static mut BackgroundColor,
    &'static mut UiBorderColor,
);

pub fn repaint_deactivated_buttons(
    theme: Option<Res<GdtfTheme>>,
    mut deactivated: RemovedComponents<ActiveButton>,
    mut buttons: Query<DeactivationVisuals, DeactivatedButton>,
) {
    let Some(theme) = theme else {
        return;
    };

    for entity in deactivated.read() {
        let Ok((interaction, mut background, mut border)) = buttons.get_mut(entity) else {
            continue;
        };
        background.0 = interaction_fill(&theme, *interaction);
        *border = UiBorderColor::all(*theme.button.border_color);
    }
}

type EnabledInteractiveButton = (
    With<Button>,
    Without<DisabledButton>,
    Without<ActiveButton>,
    Without<Segment>,
    Without<Switch>,
);

pub fn repaint_theme_change(
    theme: Res<GdtfTheme>,
    mut buttons: Query<DeactivationVisuals, EnabledInteractiveButton>,
) {
    for (interaction, mut background, mut border) in &mut buttons {
        background.0 = interaction_fill(&theme, *interaction);
        *border = UiBorderColor::all(*theme.button.border_color);
    }
}
