//! Horizontal fill progress bar.

use bevy::{
    prelude::*,
    scene::{CommandsSceneExt, bsn, template_value},
    ui::{BackgroundColor, Node, Val},
};

/// Fill amount in 0..=1.
#[derive(Deref, Clone, Copy, PartialEq, Debug)]
pub struct FillFraction(f32);

impl FillFraction {
    /// Clamp into 0..=1.
    #[must_use]
    pub const fn new(fraction: f32) -> Self {
        Self(fraction.clamp(0.0, 1.0))
    }

    /// Ratio of `current` to `max` (zero when max is non-positive).
    #[must_use]
    pub fn from_ratio(current: f32, max: f32) -> Self {
        if max <= 0.0 {
            Self(0.0)
        } else {
            Self::new(current / max)
        }
    }

    fn as_percent(self) -> Val {
        Val::Percent(self.0 * 100.0)
    }
}

/// Track (background) of a progress bar.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct ProgressBarTrack;

/// Fill child of a progress bar.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct ProgressBarFill;

/// Spawn a progress bar with the given fill and colors.
pub fn spawn_progress_bar(
    commands: &mut Commands,
    fraction: FillFraction,
    remaining: Color,
    lost: Color,
    marker: impl Bundle,
) -> Entity {
    let track_node = Node {
        width: Val::Percent(100.0),
        height: Val::Vh(BAR_HEIGHT_VH),
        ..default()
    };
    let fill_node = Node {
        width: fraction.as_percent(),
        height: Val::Percent(100.0),
        ..default()
    };
    commands
        .spawn_scene((
            bsn! {
                ProgressBarTrack
                BackgroundColor(lost)
                Children [
                    (
                        ProgressBarFill
                        BackgroundColor(remaining)
                        template_value(fill_node)
                    )
                ]
            },
            template_value(track_node),
        ))
        .insert(marker)
        .id()
}

/// Update the fill width under `track`. Returns whether a fill node was found.
pub fn set_progress_bar(
    track: Entity,
    fraction: FillFraction,
    children: &Query<&Children>,
    fills: &mut Query<&mut Node, With<ProgressBarFill>>,
) -> bool {
    let Ok(kids) = children.get(track) else {
        return false;
    };
    for &child in kids {
        if let Ok(mut node) = fills.get_mut(child) {
            node.width = fraction.as_percent();
            return true;
        }
    }
    false
}

const BAR_HEIGHT_VH: f32 = 1.38889;

#[cfg(test)]
mod test;
