//! The GTW-359 vertical-link (stair / ladder) cell draw (E7 · GTW-12k, AC4): one
//! stair- or ladder-tile sprite per authored vertical-link endpoint cell on the
//! [`ActiveLevel`].
//!
//! This module reads the sim's [`VerticalLinkGraph`](gdtf_battle_sim::VerticalLinkGraph)
//! — the authored stair / ladder links, each carrying its two `(cell, level)` endpoints
//! and its [`LinkKind`](gdtf_battle_sim::LinkKind) — for the presenter-owned
//! [`ActiveLevel`] and draws one 16x16 terrain sprite at each link endpoint on the
//! active storey, choosing the [`TileRoles`] role by the link kind AND (for a stair)
//! the active endpoint's direction within the link: a
//! [`Stair`](gdtf_battle_sim::LinkKind::Stair) endpoint draws [`TileRoles::stair_up`]
//! (atlas index `29`) when the active storey is the link's LOWER cell (you ascend) or
//! [`TileRoles::stair_down`] (atlas index `28`) when it is the UPPER cell (you descend);
//! a [`Ladder`](gdtf_battle_sim::LinkKind::Ladder) endpoint draws [`TileRoles::ladder`]
//! (atlas index `235`) for either direction.
//!
//! # Stair up/down split (GTW-373, PROPOSED semantic — flagged for in-engine confirm)
//!
//! GTW-373 (user ruling 2026-06-23) SUPERSEDES the GTW-359 OQ-3 single-`stair`-`77`
//! constant: the stair role is split into [`stair_up`](TileRoles::stair_up) /
//! [`stair_down`](TileRoles::stair_down). The PROPOSED mapping (to be confirmed
//! in-engine): render `stair_up` on a cell when the active-storey endpoint is the link's
//! LOWER cell (you ascend from here), `stair_down` when it is the UPPER cell (you descend
//! from here). "Lower" / "upper" is decided by comparing the two endpoints' storey `z`
//! (the link's `from`/`to` order is authoring order, NOT a level ordering).
//!
//! Which kind maps to which ROLE is owned HERE; which atlas INDEX a role resolves to is
//! data, read from the [`TileRoles`] resource (`assets/sprites/tile_roles.spritedef.ron`) at draw
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
//! It maintains a POOL of cell-keyed [`VerticalLinkSprite`] entities: it reuses an
//! existing sprite (re-indexing its atlas tile, moving its [`Transform`], showing it)
//! for each link cell it now needs and HIDES surplus pooled sprites it no longer needs —
//! it never despawn-then-respawns the set each frame. The walk itself is the shared
//! [`draw_pool`](crate::overlays::pool::draw_pool) helper (GTW-568), which owns the
//! `set_if_neq` visibility flips; the grow closure keeps this draw's MAY-DECLINE guard
//! (no spawn while the terrain sheet is still loading).

use bevy::{camera::visibility::RenderLayers, prelude::*};
use gdtf_battle_sim::{Cell, CellLevel, Level, LinkKind, VerticalLinkGraph};

use super::{
    active_level::ActiveLevel,
    roles::{TileIndex, TileRoles},
};
use crate::{
    CELL_PX, Layer, SheetRole, TopDownAtlases, WORLD_RENDER_LAYER, cell_to_world_layered,
    overlays::pool::draw_pool,
};

/// Marker for a pooled vertical-link (stair / ladder) tile [`Sprite`].
///
/// Plumbing around the framework sprite (the no-bare-types framework carve-out, the same
/// justification the [`PathStepSprite`](crate::PathStepSprite) /
/// [`HoverHighlight`](crate::HoverHighlight) markers use): [`draw_vertical_links`]
/// queries `With<VerticalLinkSprite>` to find and MUTATE the pooled link sprites in place
/// rather than despawn-respawning them each frame.
#[derive(Component, Debug, Default, Clone, Copy, Eq, PartialEq, Hash)]
pub struct VerticalLinkSprite;

/// The [`TileRoles`] role a vertical-link endpoint maps to — the presenter-owned mapping
/// from a link's kind AND the active endpoint's direction within the link to its
/// tile-role (C2).
///
/// A [`Stair`](LinkKind::Stair) endpoint draws the direction-keyed stair role
/// (GTW-373, PROPOSED — flagged for in-engine confirm): the active endpoint is at storey
/// `active_z`, its partner endpoint at `other_z`. If the active storey is the link's
/// LOWER cell (`active_z < other_z`) you ASCEND from here, so it draws
/// [`TileRoles::stair_up`] (atlas index `29`); otherwise the active storey is the UPPER
/// cell and you DESCEND, so it draws [`TileRoles::stair_down`] (atlas index `28`). A
/// [`Ladder`](LinkKind::Ladder) endpoint draws the single [`TileRoles::ladder`] role
/// (atlas index `235`) for either direction. The INDEX a role resolves to is read from
/// the [`TileRoles`] resource, never a literal.
const fn link_tile_index(
    kind: LinkKind,
    active_z: i32,
    other_z: i32,
    roles: &TileRoles,
) -> TileIndex {
    match kind {
        // The active storey is the link's LOWER cell -> you ascend from here (stair_up);
        // otherwise it is the UPPER cell -> you descend (stair_down).
        LinkKind::Stair { .. } if active_z < other_z => roles.stair_up,
        LinkKind::Stair { .. } => roles.stair_down,
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
///   to the endpoint's role index ([`link_tile_index`] — a stair endpoint resolves to
///   `stair_up` `29` when the active storey is the link's LOWER cell or `stair_down` `28`
///   when it is the UPPER cell; a ladder endpoint resolves to `ladder` `235`), moving it
///   to [`cell_to_world_layered`] at the [`Layer::VerticalLink`] band, and showing it;
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
        // Each endpoint's tile is direction-keyed (GTW-373): a stair endpoint resolves to
        // stair_up / stair_down by whether the ACTIVE storey is the link's lower or upper
        // cell, so the index is computed per-endpoint with its partner's storey.
        for (endpoint, other) in [(link.from, link.to), (link.to, link.from)] {
            if endpoint.z == active_z {
                let index = *link_tile_index(link.kind, endpoint.z, other.z, &roles);
                to_draw.push((endpoint, index));
            }
        }
    }

    // The world position of a link-tile sprite (shared by the reuse + grow paths).
    let world_at = |cell: CellLevel| {
        cell_to_world_layered(Cell::new(cell.x, cell.y), active_level, Layer::VerticalLink)
    };
    // The shared pooled-draw walk (GTW-568): reuse the pooled link sprites in iteration
    // order (re-index + move), lazily spawn past the pool, hide the surplus — the helper
    // owns the set_if_neq visibility flips (mutate, not respawn — C5).
    draw_pool(
        sprites.iter_mut(),
        to_draw,
        |(cell, index), (sprite, transform, _)| {
            if let Some(atlas) = sprite.texture_atlas.as_mut() {
                atlas.index = index;
            }
            transform.translation = world_at(cell);
        },
        |(cell, index)| {
            // MAY-DECLINE grow: when the terrain sheet is not loaded yet, skip the spawn
            // rather than panic — the pooled set simply stays smaller this frame.
            if let Some(sprite) = link_sprite(index, &atlases) {
                spawn_link_sprite(&mut commands, sprite, world_at(cell));
            }
        },
        |(_, _, visibility)| visibility,
    );
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

#[cfg(test)]
mod tests {
    use gdtf_battle_sim::LinkKind;

    use super::{TileRoles, link_tile_index};

    /// The SHIPPED tile-role table, deserialized (or `None` if the bytes do not parse) —
    /// the direction tests resolve their tiles against the SAME indices the running draw
    /// reads (the GTW-373 contract data), so a swapped up/down arm fails against the real
    /// `stair_up` `29` / `stair_down` `28`.
    fn shipped_roles() -> Option<TileRoles> {
        const SHIPPED: &str =
            include_str!("../../../../../assets/sprites/tile_roles.spritedef.ron");
        let parsed: Result<TileRoles, _> = ron::de::from_str(SHIPPED);
        assert!(
            parsed.is_ok(),
            "shipped tile_roles.ron must parse: {:?}",
            parsed.as_ref().err(),
        );
        parsed.ok()
    }

    /// GTW-373 (C4 (b)) — a STAIR endpoint picks `stair_up` (29) when the active storey is
    /// the link's LOWER cell (you ascend) and `stair_down` (28) when it is the UPPER cell
    /// (you descend). Pin-DISCRIMINATING: it FAILS if the up/down arms are swapped, since
    /// `stair_up` (29) != `stair_down` (28).
    #[test]
    fn stair_endpoint_picks_up_when_lower_down_when_upper() {
        let Some(roles) = shipped_roles() else { return };
        let stair = LinkKind::stair();

        // Active storey is the LOWER cell (active_z 0 < other_z 1): ASCEND -> stair_up (29).
        assert_eq!(
            *link_tile_index(stair, 0, 1, &roles),
            29,
            "a stair endpoint on the link's LOWER cell (you ascend) must draw stair_up (29)",
        );
        // Active storey is the UPPER cell (active_z 1 > other_z 0): DESCEND -> stair_down (28).
        assert_eq!(
            *link_tile_index(stair, 1, 0, &roles),
            28,
            "a stair endpoint on the link's UPPER cell (you descend) must draw stair_down (28)",
        );
        // The two arms are DISTINCT — a swap would make these equal, so this discriminates.
        assert_ne!(
            *link_tile_index(stair, 0, 1, &roles),
            *link_tile_index(stair, 1, 0, &roles),
            "the ascend and descend stair tiles must be distinct (up 29 vs down 28)",
        );
    }

    /// GTW-373 (C4 (b) cont.) — a LADDER endpoint draws the single `ladder` (235) tile for
    /// EITHER direction (a ladder is not split up/down).
    #[test]
    fn ladder_endpoint_draws_ladder_either_direction() {
        let Some(roles) = shipped_roles() else { return };
        let ladder = LinkKind::ladder();
        assert_eq!(
            *link_tile_index(ladder, 0, 1, &roles),
            235,
            "a ladder endpoint on the lower cell draws ladder (235)",
        );
        assert_eq!(
            *link_tile_index(ladder, 1, 0, &roles),
            235,
            "a ladder endpoint on the upper cell draws ladder (235)",
        );
    }
}
