use bevy::{
    prelude::*,
    scene::{CommandsSceneExt, bsn, template_value},
    ui::{BackgroundColor, BorderRadius, Node, Val},
};

#[derive(Deref, Clone, Copy, PartialEq, Eq, Debug)]
pub struct FilledPips(usize);

impl FilledPips {
        #[must_use]
    pub const fn new(filled: usize) -> Self {
        Self(filled)
    }
}

#[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct PipsRow;

#[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Pip;

pub fn spawn_pips(
    commands: &mut Commands,
    total: usize,
    filled: FilledPips,
    remaining: Color,
    lost: Color,
    marker: impl Bundle,
) -> Entity {
    let row_node = Node {
        column_gap: Val::Vw(PIP_GAP_VW),
        ..default()
    };
    let pip_node = Node {
        width: Val::Vw(PIP_DIAMETER_VW),
        height: Val::Vw(PIP_DIAMETER_VW),
        border_radius: BorderRadius::all(Val::Percent(50.0)),
        ..default()
    };
    let pips: Vec<_> = (0..total)
        .map(|index| {
            let color = if index < *filled { remaining } else { lost };
            let node = pip_node.clone();
            bsn! {
                Pip
                BackgroundColor(color)
                template_value(node)
            }
        })
        .collect();
    commands
        .spawn_scene((
            bsn! {
                PipsRow
                Children [ { pips } ]
            },
            template_value(row_node),
        ))
        .insert(marker)
        .id()
}

pub fn set_pips(
    row: Entity,
    filled: FilledPips,
    remaining: Color,
    lost: Color,
    children: &Query<&Children>,
    pips: &mut Query<&mut BackgroundColor, With<Pip>>,
) -> usize {
    let Ok(kids) = children.get(row) else {
        return 0;
    };
    let mut recolored = 0usize;
    for (index, child) in kids.iter().enumerate() {
        if let Ok(mut background) = pips.get_mut(child) {
            background.0 = if index < *filled { remaining } else { lost };
            recolored += 1;
        }
    }
    recolored
}

const PIP_DIAMETER_VW: f32 = 0.9375;

const PIP_GAP_VW: f32 = 0.3125;

#[cfg(test)]
mod test;
