use bevy::{input_focus::InputFocus, prelude::*};
use gdtf_battle_input::PanelNavOrder;

#[derive(Deref, Clone, Copy, Debug)]
struct FocusRingPx(f32);

impl FocusRingPx {
    const WIDTH: Self = Self(2.0);
    const OFFSET: Self = Self(1.0);
}

const FOCUS_RING_COLOR: Color = Color::srgb(1.0, 0.82, 0.2);

pub(in crate::states::running::game::battlescape) fn paint_focus_outline(
    mut commands: Commands,
    focus: Res<InputFocus>,
    buttons: Query<(Entity, Has<Outline>), With<PanelNavOrder>>,
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
