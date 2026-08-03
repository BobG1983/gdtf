use bevy::{
    prelude::*,
    ui::{BackgroundColor, BorderColor as UiBorderColor, Interaction, widget::Button},
};

use crate::{
    theme::GdtfTheme,
    widgets::core::{ActiveButton, DisabledButton, Segment, Switch},
};

type InteractedButton = (
    Changed<Interaction>,
    With<Button>,
    Without<DisabledButton>,
    Without<ActiveButton>,
    Without<Segment>,
    Without<Switch>,
);

type InteractionVisuals = (
    &'static Interaction,
    &'static mut BackgroundColor,
    &'static mut UiBorderColor,
);

pub(crate) fn interaction_fill(theme: &GdtfTheme, interaction: Interaction) -> Color {
    match interaction {
        Interaction::None => *theme.button.color,
        Interaction::Hovered => *theme.button.hover,
        Interaction::Pressed => *theme.button.pressed,
    }
}

pub fn theme_interaction(
    theme: Option<Res<GdtfTheme>>,
    mut buttons: Query<InteractionVisuals, InteractedButton>,
) {
    let Some(theme) = theme else {
        return;
    };

    for (interaction, mut background, mut border) in &mut buttons {
        background.0 = interaction_fill(&theme, *interaction);
        *border = UiBorderColor::all(*theme.button.border_color);
    }
}
