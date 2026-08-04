//! Toggle switch widget.

use bevy::{
    prelude::*,
    scene::{CommandsSceneExt, bsn, template_value},
    ui::{
        AlignItems, BackgroundColor, BorderRadius, Interaction, JustifyContent, Node, UiRect, Val,
        widget::Button,
    },
};

use super::orientation::Orientation;

/// On/off state of a switch.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum SwitchState {
    /// Off.
    #[default]
    Off,
    /// On.
    On,
}

impl SwitchState {
    /// Toggle to the other state.
    #[must_use]
    pub const fn flipped(self) -> Self {
        match self {
            Self::Off => Self::On,
            Self::On => Self::Off,
        }
    }

    /// Whether the switch is on.
    #[must_use]
    pub const fn is_on(self) -> bool {
        matches!(self, Self::On)
    }
}

/// Colors for track and knob.
#[derive(Component, Clone, Copy, PartialEq, Debug, Default)]
pub struct SwitchColors {
    /// Track when off.
    pub off:  Color,
    /// Track when on.
    pub on:   Color,
    /// Knob color.
    pub knob: Color,
}

impl SwitchColors {
    /// Track color for `state`.
    #[must_use]
    pub const fn track(&self, state: SwitchState) -> Color {
        match state {
            SwitchState::Off => self.off,
            SwitchState::On => self.on,
        }
    }
}

/// Marks the switch root.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Switch;

/// Marks the knob child.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct SwitchKnob;

/// Axis the switch is laid out on.
#[derive(Component, Deref, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct SwitchOrientation(Orientation);

/// Emitted when a switch is toggled by the user.
#[derive(Message, Clone, Copy, PartialEq, Eq, Debug)]
pub struct ToggleFlipped {
    /// Switch entity.
    pub switch: Entity,
    /// New state after the flip.
    pub state:  SwitchState,
}

/// Spawn a switch with the given state, colors, and orientation.
pub fn spawn_switch(
    commands: &mut Commands,
    state: SwitchState,
    colors: SwitchColors,
    orientation: Orientation,
    marker: impl Bundle,
) -> Entity {
    let (width, height) = match orientation {
        Orientation::Horizontal => (Val::Vw(TRACK_LONG_VW), Val::Vw(TRACK_SHORT_VW)),
        Orientation::Vertical => (Val::Vw(TRACK_SHORT_VW), Val::Vw(TRACK_LONG_VW)),
    };
    let track_node = Node {
        width,
        height,
        flex_direction: orientation.flex_direction(),
        padding: UiRect::all(Val::Vw(TRACK_PAD_VW)),
        align_items: AlignItems::Center,
        justify_content: knob_justify(state),
        border_radius: BorderRadius::all(Val::Percent(50.0)),
        ..default()
    };
    let knob_node = Node {
        width: Val::Vw(KNOB_DIAMETER_VW),
        height: Val::Vw(KNOB_DIAMETER_VW),
        border_radius: BorderRadius::all(Val::Percent(50.0)),
        ..default()
    };
    let switch_orientation = SwitchOrientation(orientation);
    let track_color = colors.track(state);
    let knob_color = colors.knob;
    commands
        .spawn_scene((
            bsn! {
                Switch
                Button
                BackgroundColor(track_color)
                Children [
                    (
                        SwitchKnob
                        BackgroundColor(knob_color)
                        template_value(knob_node)
                    )
                ]
            },
            template_value(switch_orientation),
            template_value(state),
            template_value(colors),
            template_value(track_node),
        ))
        .insert(marker)
        .id()
}

type SwitchData = (
    Entity,
    &'static Interaction,
    &'static mut SwitchState,
    &'static SwitchColors,
    &'static SwitchOrientation,
    &'static mut Node,
);

/// Flip switches on press and emit [`ToggleFlipped`].
pub fn drive_switches(
    mut switches: Query<SwitchData, (Changed<Interaction>, With<Switch>)>,
    mut backgrounds: Query<&mut BackgroundColor, With<Switch>>,
    mut flipped: MessageWriter<ToggleFlipped>,
) {
    for (entity, interaction, mut state, colors, orientation, mut node) in &mut switches {
        if *interaction != Interaction::Pressed {
            continue;
        }
        let next = state.flipped();
        *state = next;
        node.flex_direction = orientation.flex_direction();
        node.justify_content = knob_justify(next);
        if let Ok(mut background) = backgrounds.get_mut(entity) {
            background.0 = colors.track(next);
        }
        flipped.write(ToggleFlipped {
            switch: entity,
            state:  next,
        });
    }
}

const fn knob_justify(state: SwitchState) -> JustifyContent {
    match state {
        SwitchState::Off => JustifyContent::FlexStart,
        SwitchState::On => JustifyContent::FlexEnd,
    }
}

const TRACK_LONG_VW: f32 = 3.75;

const TRACK_SHORT_VW: f32 = 1.562_5;

const TRACK_PAD_VW: f32 = 0.234_375;

const KNOB_DIAMETER_VW: f32 = 1.09375;

#[cfg(test)]
mod test;
