//! Hover-cell highlight sprite.

use bevy::{
    camera::visibility::RenderLayers,
    ecs::template::template,
    prelude::*,
    scene::{CommandsSceneExt, bsn, template_value},
};
use gdtf_battle_sim::prelude::CellLevel;

use crate::{CELL_PX, CellVisibility, WORLD_RENDER_LAYER, cell_to_world};

/// Request to show or clear the hover highlight on a cell.
#[derive(Message, Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct HighlightRequest {
    #[deref]
    cell:       Option<CellLevel>,
    visibility: CellVisibility,
}

impl HighlightRequest {
    /// Build a highlight request for an optional cell and its visibility.
    #[must_use]
    pub const fn new(cell: Option<CellLevel>, visibility: CellVisibility) -> Self {
        Self { cell, visibility }
    }

    /// Squad-visibility of the requested cell.
    #[must_use]
    pub const fn visibility(self) -> CellVisibility {
        self.visibility
    }
}

/// Marker on the single hover-highlight sprite entity.
#[derive(Component, Debug, Default, Clone, Copy, Eq, PartialEq, Hash)]
pub struct HoverHighlight;

const HIGHLIGHT_TINT: Color = Color::srgba(1.0, 0.95, 0.6, 0.35);

const UNSEEN_TINT: Color = Color::srgba(0.55, 0.6, 0.7, 0.35);

const fn tint_for(visibility: CellVisibility) -> Color {
    if visibility.is_squad_visible() {
        HIGHLIGHT_TINT
    } else {
        UNSEEN_TINT
    }
}

/// Apply the latest [`HighlightRequest`] to the hover sprite (spawn if needed).
pub fn draw_highlight_on_request(
    mut commands: Commands,
    mut requests: MessageReader<HighlightRequest>,
    mut highlights: Query<(&mut Transform, &mut Sprite, &mut Visibility), With<HoverHighlight>>,
) {
    let Some(request) = requests.read().last().copied() else {
        return;
    };

    let target = (*request).map(|cell| {
        let (cell, level) = cell.split();
        cell_to_world(cell, level)
    });
    let tint = tint_for(request.visibility());

    match highlights.single_mut() {
        Ok((mut transform, mut sprite, mut visibility)) => match target {
            Some(world) => {
                transform.translation = world;
                sprite.color = tint;
                *visibility = Visibility::Visible;
            }
            None => *visibility = Visibility::Hidden,
        },
        Err(_) => {
            if let Some(world) = target {
                let sprite = Sprite {
                    color: tint,
                    custom_size: Some(Vec2::splat(CELL_PX)),
                    ..default()
                };
                let transform = Transform::from_translation(world);
                let visibility = Visibility::Visible;
                let layers = RenderLayers::layer(WORLD_RENDER_LAYER);
                commands.spawn_scene((
                    bsn! {
                        HoverHighlight
                        template(move |_| Ok(sprite.clone()))
                    },
                    template_value(transform),
                    template_value(visibility),
                    template_value(layers),
                ));
            }
        }
    }
}
