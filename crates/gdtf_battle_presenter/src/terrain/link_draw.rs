//! The GTW-359 vertical-link (stair / ladder) cell draw (E7 · GTW-12k, AC4): one
//! stair- or ladder-tile sprite per authored vertical-link endpoint cell on the
//! [`ActiveLevel`].
//!
//! This module reads the sim's [`VerticalLinkGraph`](gdtf_battle_sim::VerticalLinkGraph)
//! — the authored stair / ladder links, each carrying its two `(cell, level)` endpoints
//! and its [`LinkKind`](gdtf_battle_sim::LinkKind) — for the presenter-owned
//! [`ActiveLevel`] and draws one 16x16 terrain sprite at each link endpoint on the
//! active storey, choosing the [`TileRoles`] role by the link kind: a
//! [`Stair`](gdtf_battle_sim::LinkKind::Stair) endpoint draws [`TileRoles::stair`]
//! (atlas index `77`), a [`Ladder`](gdtf_battle_sim::LinkKind::Ladder) endpoint draws
//! [`TileRoles::ladder`] (atlas index `235`).
//!
//! Which kind maps to which ROLE is owned HERE; which atlas INDEX a role resolves to is
//! data, read from the [`TileRoles`] resource (`assets/tiles/tile_roles.ron`) at draw
//! time — never a hardcoded literal. The model never reads the presenter (ADR-0001).
//!
//! # Hard-cut to the active storey (AC4)
//!
//! A vertical link has two endpoints on adjacent storeys; this draw renders ONLY the
//! endpoint whose `z` equals the active storey (the SAME hard cut the static terrain
//! draw + the reachable overlay use). An off-storey link cell is not drawn.
//!
//! # Mutate, never respawn (C5)
//!
//! Like [`draw_reachable_overlay`](crate::draw_reachable_overlay), it maintains a POOL of
//! cell-keyed [`VerticalLinkSprite`] entities: it reuses an existing sprite (re-indexing
//! its atlas tile, moving its [`Transform`], showing it) for each link cell it now needs
//! and HIDES surplus pooled sprites it no longer needs — it never despawn-then-respawns
//! the set each frame (the mirror of [`draw_reachable_overlay`] / [`present_fog`](crate::present_fog)).

use bevy::{camera::visibility::RenderLayers, prelude::*};
use gdtf_battle_sim::{Cell, CellLevel, Level, LinkKind, VerticalLinkGraph};

use super::{
    active_level::ActiveLevel,
    roles::{TileIndex, TileRoles},
};
use crate::{CELL_PX, Layer, SheetRole, TopDownAtlases, WORLD_RENDER_LAYER, cell_to_world_layered};

/// Marker for a pooled vertical-link (stair / ladder) tile [`Sprite`].
///
/// Plumbing around the framework sprite (the no-bare-types framework carve-out, the same
/// justification the [`ReachableTint`](crate::ReachableTint) /
/// [`HoverHighlight`](crate::HoverHighlight) markers use): [`draw_vertical_links`]
/// queries `With<VerticalLinkSprite>` to find and MUTATE the pooled link sprites in place
/// rather than despawn-respawning them each frame.
#[derive(Component, Debug, Default, Clone, Copy, Eq, PartialEq, Hash)]
pub struct VerticalLinkSprite;

/// The [`TileRoles`] role a [`LinkKind`] maps to — the presenter-owned mapping from a
/// vertical link's kind to its tile-role (C2).
///
/// A [`Stair`](LinkKind::Stair) draws the [`TileRoles::stair`] role (atlas index `77`); a
/// [`Ladder`](LinkKind::Ladder) draws the [`TileRoles::ladder`] role (atlas index `235`).
/// The INDEX a role resolves to is read from the [`TileRoles`] resource, never a literal.
const fn link_tile_index(kind: LinkKind, roles: &TileRoles) -> TileIndex {
    match kind {
        LinkKind::Stair { .. } => roles.stair,
        LinkKind::Ladder { .. } => roles.ladder,
    }
}

/// Build one vertical-link [`Sprite`] on the TERRAIN sheet at the role's [`TileIndex`].
///
/// `Sprite::from_atlas_image(terrain.image, TextureAtlas { layout, index })` with
/// `custom_size = Some(Vec2::splat(CELL_PX))` (the S3/S4 sizing recipe) — the SAME
/// atlas-sprite recipe the ganger draw uses, on the terrain sheet (stair / ladder are
/// terrain tiles). Returns [`None`] if the terrain sheet was not loaded (so the caller
/// skips the spawn rather than panic).
fn link_sprite(index: usize, atlases: &TopDownAtlases) -> Option<Sprite> {
    let terrain = atlases.role(SheetRole::Terrain)?;
    let mut sprite = Sprite::from_atlas_image(
        terrain.image.clone(),
        TextureAtlas {
            layout: terrain.layout.clone(),
            index,
        },
    );
    sprite.custom_size = Some(Vec2::splat(CELL_PX));
    Some(sprite)
}

/// `Update` ([`PresenterSystems::Draw`](crate::PresenterSystems)): draw the
/// vertical-link (stair / ladder) tiles — one cell-keyed [`Sprite`] per authored link
/// endpoint on the active storey (AC4).
///
/// Reads [`Res<VerticalLinkGraph>`](gdtf_battle_sim::VerticalLinkGraph),
/// [`Res<ActiveLevel>`](crate::ActiveLevel), [`Res<TileRoles>`], and
/// [`Res<TopDownAtlases>`], then maintains a POOL of [`VerticalLinkSprite`] sprites:
///
/// - for each authored link, it draws the endpoint cell ON the active storey (the hard
///   cut, AC4) — taking (or lazily spawning) a pooled sprite, re-indexing its atlas tile
///   to the link kind's role index ([`link_tile_index`] — stair `77` / ladder `235`),
///   moving it to [`cell_to_world_layered`] at the [`Layer::VerticalLink`] band, and
///   showing it;
/// - every surplus pooled sprite (links not on this storey, or pooled entities beyond
///   the current set) is [`Visibility::Hidden`] — NEVER despawned (mutate, not respawn,
///   C5).
///
/// A link's two endpoints sit on adjacent storeys; iterating the GRAPH's links (not
/// [`links_from`](gdtf_battle_sim::VerticalLinkGraph::links_from)) once per link and
/// filtering each endpoint to the active storey draws exactly the on-storey endpoint —
/// `from` when its storey is active, `to` when its storey is active.
///
/// Param-only (`bevy-traps.md` #7): [`Commands`] for the lazy pool growth, the
/// [`VerticalLinkGraph`](gdtf_battle_sim::VerticalLinkGraph) / [`ActiveLevel`] /
/// [`TileRoles`] / [`TopDownAtlases`] reads, and a
/// `Query<(&mut Sprite, &mut Transform, &mut Visibility), With<VerticalLinkSprite>>` for
/// the in-place re-index + move + show/hide. Battle-gated + in
/// [`PresenterSystems::Draw`](crate::PresenterSystems) by the
/// [`TopDownRendererPlugin`](crate::TopDownRendererPlugin).
pub fn draw_vertical_links(
    mut commands: Commands,
    graph: Res<VerticalLinkGraph>,
    active: Res<ActiveLevel>,
    roles: Res<TileRoles>,
    atlases: Res<TopDownAtlases>,
    mut sprites: Query<(&mut Sprite, &mut Transform, &mut Visibility), With<VerticalLinkSprite>>,
) {
    let active_level: Level = **active;
    let active_z = i32::from(*active_level);

    // The on-storey link endpoints to draw THIS update — for each authored link, the
    // endpoint cell whose z == the active storey, paired with the link kind's tile index.
    // Iterating the links (not `links_from`) draws each link once; the per-endpoint storey
    // filter is the AC4 hard cut (off-storey endpoints are not drawn).
    let mut to_draw: Vec<(CellLevel, usize)> = Vec::new();
    for link in graph.links() {
        let index = *link_tile_index(link.kind, &roles);
        for endpoint in [link.from, link.to] {
            if endpoint.z == active_z {
                to_draw.push((endpoint, index));
            }
        }
    }

    // Reuse the pooled link sprites in iteration order: re-index + move + show the first
    // `to_draw.len()`, hide the rest (mutate, not respawn — C5).
    let mut pooled = sprites.iter_mut();
    for (cell, index) in &to_draw {
        let world =
            cell_to_world_layered(Cell::new(cell.x, cell.y), active_level, Layer::VerticalLink);
        if let Some((mut sprite, mut transform, mut visibility)) = pooled.next() {
            if let Some(atlas) = sprite.texture_atlas.as_mut() {
                atlas.index = *index;
            }
            transform.translation = world;
            *visibility = Visibility::Visible;
        } else if let Some(sprite) = link_sprite(*index, &atlases) {
            spawn_link_sprite(&mut commands, sprite, world);
        }
    }
    // Hide every surplus pooled sprite the current set no longer needs.
    for (_, _, mut visibility) in pooled {
        *visibility = Visibility::Hidden;
    }
}

/// Lazily spawn ONE pooled vertical-link tile sprite at `world`.
///
/// The `sprite` (an atlas tile already indexed to the link kind's role) on the world
/// render layer at the [`Layer::VerticalLink`] band, shown from spawn. Pooled (kept +
/// reused / hidden, never despawned), so this runs only when the on-storey link set grows
/// past the current pool size.
fn spawn_link_sprite(commands: &mut Commands, sprite: Sprite, world: Vec3) {
    commands.spawn((
        VerticalLinkSprite,
        sprite,
        Transform::from_translation(world),
        Visibility::Visible,
        RenderLayers::layer(WORLD_RENDER_LAYER),
    ));
}
