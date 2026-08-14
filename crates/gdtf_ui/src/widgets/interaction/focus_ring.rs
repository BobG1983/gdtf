//! Ring the button holding keyboard or gamepad focus, on any screen.

use bevy::{input_focus::InputFocus, prelude::*, ui::widget::Button};

use crate::widgets::core::DisabledButton;

#[derive(Deref, Clone, Copy, Debug)]
struct FocusRingPx(f32);

impl FocusRingPx {
    const OFFSET: Self = Self(1.0);
    const WIDTH: Self = Self(2.0);
}

const FOCUS_RING_COLOR: Color = Color::srgb(1.0, 0.82, 0.2);

type RingableButton = (With<Button>, Without<DisabledButton>);

/// Outline the focused button and clear the outline from one that lost focus.
pub fn paint_focus_ring(
    mut commands: Commands,
    focus: Res<InputFocus>,
    buttons: Query<(Entity, Has<Outline>), RingableButton>,
) {
    let focused = focus.get();
    for (entity, has_ring) in &buttons {
        match (Some(entity) == focused, has_ring) {
            (true, false) => {
                commands.entity(entity).insert(Outline {
                    width:  Val::Px(*FocusRingPx::WIDTH),
                    offset: Val::Px(*FocusRingPx::OFFSET),
                    color:  FOCUS_RING_COLOR,
                });
            }
            (false, true) => {
                commands.entity(entity).remove::<Outline>();
            }
            _ => {}
        }
    }
}
