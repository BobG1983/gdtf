//! Stair and ladder endpoint sprites on the active storey.

use bevy::{camera::visibility::RenderLayers, prelude::*};
use gdtf_battle_sim::{
    def::TerrainDefRegistry,
    piece::TerrainGraphicKey,
    prelude::{CellLevel, Level},
    vertical::{LinkKind, VerticalLinkGraph},
};

use super::{
    active_level::ActiveLevel,
    static_map::SpriteResolveCtx,
    view_resolve::{LinkEnd, TerrainPieces, view_key_at},
};
use crate::{Layer, WORLD_RENDER_LAYER, cell_to_world_layered, overlays::pool::draw_pool};

/// Marker on a vertical-link endpoint sprite.
#[derive(Component, Debug, Default, Clone, Copy, Eq, PartialEq, Hash)]
pub struct VerticalLinkSprite;

/// The one sprite record render code still names: a ladder is drawn from the link kind.
const LADDER_SPRITE: &str = "ladder";

// A ladder draws its own marker; a stair asks the piece standing at the endpoint.
fn endpoint_key(
    kind: LinkKind,
    endpoint: CellLevel,
    other: CellLevel,
    pieces: &TerrainPieces,
    defs: &TerrainDefRegistry,
) -> Option<TerrainGraphicKey> {
    match kind {
        LinkKind::Ladder { .. } => Some(TerrainGraphicKey::new(LADDER_SPRITE.to_owned())),
        LinkKind::Stair { .. } => {
            let end = if endpoint.z < other.z {
                LinkEnd::Lower
            } else {
                LinkEnd::Upper
            };
            view_key_at(endpoint, pieces, defs, Some(end)).cloned()
        }
    }
}

/// Draw stair/ladder markers for every link endpoint on the active storey.
pub fn draw_vertical_links(
    mut commands: Commands,
    graph: Res<VerticalLinkGraph>,
    active: Res<ActiveLevel>,
    defs: Res<TerrainDefRegistry>,
    pieces: TerrainPieces,
    resolve: SpriteResolveCtx,
    mut sprites: Query<(&mut Sprite, &mut Transform, &mut Visibility), With<VerticalLinkSprite>>,
) {
    let active_level: Level = **active;
    let active_z = i32::from(*active_level);

    let mut to_draw: Vec<(CellLevel, TerrainGraphicKey)> = Vec::new();
    for link in graph.links() {
        for (endpoint, other) in [(link.from, link.to), (link.to, link.from)] {
            if endpoint.z != active_z {
                continue;
            }
            if let Some(key) = endpoint_key(link.kind, endpoint, other, &pieces, &defs) {
                to_draw.push((endpoint, key));
            }
        }
    }

    let world_at = |cell: CellLevel, offset: Vec2| {
        cell_to_world_layered(cell.cell(), active_level, Layer::VerticalLink) + offset.extend(0.0)
    };
    draw_pool(
        sprites.iter_mut(),
        to_draw,
        |(cell, key), (sprite, transform, _)| {
            let (resolved, offset) = resolve.resolved_sprite(&key, &cell);
            **sprite = resolved;
            transform.translation = world_at(cell, offset);
        },
        |(cell, key)| {
            let (resolved, offset) = resolve.resolved_sprite(&key, &cell);
            spawn_link_sprite(&mut commands, resolved, world_at(cell, offset));
        },
        |(_, _, visibility)| visibility,
    );
}

fn spawn_link_sprite(commands: &mut Commands, sprite: Sprite, world: Vec3) {
    commands.spawn((
        VerticalLinkSprite,
        sprite,
        Transform::from_translation(world),
        Visibility::Visible,
        RenderLayers::layer(WORLD_RENDER_LAYER),
    ));
}
