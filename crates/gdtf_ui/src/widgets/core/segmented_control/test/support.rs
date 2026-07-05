//! Shared segmented-control-only test fixtures: the `FireMode` identity
//! marker, the segment/look probes, the discriminating `SEG_COLORS` palette,
//! and the 3-segment fire-mode spawner.

use bevy::{
    ecs::system::SystemState,
    prelude::*,
    text::{FontWeight, TextColor as UiTextColor, TextFont},
    ui::BackgroundColor,
};

use super::super::{
    Segment, SegmentColors, SegmentIndex, SegmentLabel, SegmentText, spawn_segmented_control,
};
use crate::widgets::core::Orientation;

/// A caller-attached identity marker on a segmented control.
#[derive(Component, Clone, Copy)]
pub(super) struct FireMode;

/// A four-color segment palette with all four colors distinct so asserts discriminate.
pub(super) const SEG_COLORS: SegmentColors = SegmentColors {
    active_bg:   Color::srgb(0.2, 0.7, 0.2),
    active_text: Color::srgb(1.0, 1.0, 1.0),
    base_bg:     Color::srgb(0.1, 0.1, 0.1),
    base_text:   Color::srgb(0.5, 0.5, 0.5),
};

/// The segments of the control rooted at `control`, ordered by child order, as
/// `(entity, index)`.
pub(super) fn segments_of(app: &mut App, control: Entity) -> Vec<(Entity, usize)> {
    let mut state: SystemState<Query<&Children>> = SystemState::new(app.world_mut());
    let Ok(children) = state.get(app.world()) else {
        return Vec::new();
    };
    let Ok(kids) = children.get(control) else {
        return Vec::new();
    };
    kids.iter()
        .filter_map(|c| {
            app.world()
                .get::<Segment>(c)
                .and(app.world().get::<SegmentIndex>(c))
                .map(|idx| (c, **idx))
        })
        .collect()
}

/// The (background color, label weight, label color) of a segment entity.
pub(super) fn segment_look(app: &mut App, segment: Entity) -> (Color, FontWeight, Color) {
    let bg = app
        .world()
        .get::<BackgroundColor>(segment)
        .map_or(Color::NONE, |c| c.0);
    // Find the label text child.
    let label = {
        let mut state: SystemState<Query<&Children>> = SystemState::new(app.world_mut());
        let children = state.get(app.world());
        children.ok().and_then(|children| {
            children.get(segment).ok().and_then(|kids| {
                kids.iter()
                    .find(|&c| app.world().get::<SegmentText>(c).is_some())
            })
        })
    };
    let (weight, color) = label.map_or((FontWeight::NORMAL, Color::NONE), |l| {
        let w = app
            .world()
            .get::<TextFont>(l)
            .map_or(FontWeight::NORMAL, |f| f.weight);
        let c = app
            .world()
            .get::<UiTextColor>(l)
            .map_or(Color::NONE, |c| c.0);
        (w, c)
    });
    (bg, weight, color)
}

/// Builds a 3-segment fire-mode control with segment 0 (`Single`) active.
pub(super) fn spawn_fire_mode(app: &mut App) -> Entity {
    let labels = [
        SegmentLabel::new("Single"),
        SegmentLabel::new("Burst"),
        SegmentLabel::new("Full-Auto"),
    ];
    let control = {
        let mut commands = app.world_mut().commands();
        spawn_segmented_control(
            &mut commands,
            &labels,
            0,
            SEG_COLORS,
            Orientation::Horizontal,
            FireMode,
        )
    };
    app.world_mut().flush();
    control
}
