//! stair- or ladder-tile sprite per authored vertical-link endpoint cell on the
//! — the authored stair / ladder links, each carrying its two `(cell, level)` endpoints
use bevy::{camera::visibility::RenderLayers, prelude::*};
use gdtf_battle_sim::{
    prelude::{CellLevel, Level},
    vertical::{LinkKind, VerticalLinkGraph},
};

use super::{active_level::ActiveLevel, roles::TileRole, static_map::SpriteResolveCtx};
use crate::{Layer, WORLD_RENDER_LAYER, cell_to_world_layered, overlays::pool::draw_pool};

#[derive(Component, Debug, Default, Clone, Copy, Eq, PartialEq, Hash)]
pub struct VerticalLinkSprite;

const fn link_tile_role(kind: LinkKind, active_z: i32, other_z: i32) -> TileRole {
    match kind {
        LinkKind::Stair { .. } if active_z < other_z => TileRole::StairUp,
        LinkKind::Stair { .. } => TileRole::StairDown,
        LinkKind::Ladder { .. } => TileRole::Ladder,
    }
}

pub fn draw_vertical_links(
    mut commands: Commands,
    graph: Res<VerticalLinkGraph>,
    active: Res<ActiveLevel>,
    resolve: SpriteResolveCtx,
    mut sprites: Query<(&mut Sprite, &mut Transform, &mut Visibility), With<VerticalLinkSprite>>,
) {
    let active_level: Level = **active;
    let active_z = i32::from(*active_level);

    let mut to_draw: Vec<(CellLevel, TileRole)> = Vec::new();
    for link in graph.links() {
        for (endpoint, other) in [(link.from, link.to), (link.to, link.from)] {
            if endpoint.z == active_z {
                to_draw.push((endpoint, link_tile_role(link.kind, endpoint.z, other.z)));
            }
        }
    }

    let world_at = |cell: CellLevel, offset: Vec2| {
        cell_to_world_layered(cell.cell(), active_level, Layer::VerticalLink) + offset.extend(0.0)
    };
    draw_pool(
        sprites.iter_mut(),
        to_draw,
        |(cell, role), (sprite, transform, _)| {
            let (resolved, offset) = resolve.resolved_sprite(role.as_key(), &cell);
            **sprite = resolved;
            transform.translation = world_at(cell, offset);
        },
        |(cell, role)| {
            let (resolved, offset) = resolve.resolved_sprite(role.as_key(), &cell);
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

#[cfg(test)]
mod tests {
    use gdtf_battle_sim::vertical::LinkKind;

    use super::{TileRole, link_tile_role};

                        #[test]
    fn stair_endpoint_picks_up_when_lower_down_when_upper() {
        let stair = LinkKind::stair();

        assert_eq!(
            link_tile_role(stair, 0, 1),
            TileRole::StairUp,
            "a stair endpoint on the link's LOWER cell (you ascend) must draw stair_up",
        );
        assert_eq!(
            link_tile_role(stair, 1, 0),
            TileRole::StairDown,
            "a stair endpoint on the link's UPPER cell (you descend) must draw stair_down",
        );
    }

            #[test]
    fn ladder_endpoint_draws_ladder_either_direction() {
        let ladder = LinkKind::ladder();
        assert_eq!(
            link_tile_role(ladder, 0, 1),
            TileRole::Ladder,
            "a ladder endpoint on the lower cell draws the ladder role",
        );
        assert_eq!(
            link_tile_role(ladder, 1, 0),
            TileRole::Ladder,
            "a ladder endpoint on the upper cell draws the ladder role",
        );
    }
}
