//! The GTW-359 vertical-link (stair / ladder) cell draw (E7 · GTW-12k, AC4): one
//! stair- or ladder-tile sprite per authored vertical-link endpoint cell on the
//! [`ActiveLevel`].
//!
//! This module reads the sim's [`VerticalLinkGraph`](gdtf_battle_sim::vertical::VerticalLinkGraph)
//! — the authored stair / ladder links, each carrying its two `(cell, level)` endpoints
//! and its [`LinkKind`](gdtf_battle_sim::vertical::LinkKind) — for the presenter-owned
//! [`ActiveLevel`] and draws one 16x16 terrain sprite at each link endpoint on the
//! active storey, choosing the [`TileRole`] by the link kind AND (for a stair)
//! the active endpoint's direction within the link: a
//! [`Stair`](gdtf_battle_sim::vertical::LinkKind::Stair) endpoint draws [`TileRole::StairUp`]
//! when the active storey is the link's LOWER cell (you ascend) or
//! [`TileRole::StairDown`] when it is the UPPER cell (you descend);
//! a [`Ladder`](gdtf_battle_sim::vertical::LinkKind::Ladder) endpoint draws [`TileRole::Ladder`]
//! for either direction.
//!
//! # Stair up/down split (GTW-373, PROPOSED semantic — flagged for in-engine confirm)
//!
//! GTW-373 (user ruling 2026-06-23) SUPERSEDES the GTW-359 OQ-3 single-`stair`-`77`
//! constant: the stair role is split into [`TileRole::StairUp`] /
//! [`TileRole::StairDown`]. The PROPOSED mapping (to be confirmed
//! in-engine): render `stair_up` on a cell when the active-storey endpoint is the link's
//! LOWER cell (you ascend from here), `stair_down` when it is the UPPER cell (you descend
//! from here). "Lower" / "upper" is decided by comparing the two endpoints' storey `z`
//! (the link's `from`/`to` order is authoring order, NOT a level ordering).
//!
//! Which kind maps to which ROLE is owned HERE; which PIXELS a role resolves to is
//! data — the role key's sprite def (`assets/content/sprites/<key>.spritedef.ron`),
//! resolved through the GTW-665 [`SpriteResolveCtx`] at draw
//! time — never a hardcoded index. The model never reads the presenter (ADR-0001).
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
//! existing sprite (retargeting its image/rect, moving its [`Transform`], showing it)
//! for each link cell it now needs and HIDES surplus pooled sprites it no longer needs —
//! it never despawn-then-respawns the set each frame. The walk itself is the shared
//! [`draw_pool`](crate::overlays::pool::draw_pool) helper (GTW-568), which owns the
//! `set_if_neq` visibility flips.

use bevy::{camera::visibility::RenderLayers, prelude::*};
use gdtf_battle_sim::{
    prelude::{CellLevel, Level},
    vertical::{LinkKind, VerticalLinkGraph},
};

use super::{active_level::ActiveLevel, roles::TileRole, static_map::SpriteResolveCtx};
use crate::{Layer, WORLD_RENDER_LAYER, cell_to_world_layered, overlays::pool::draw_pool};

/// Marker for a pooled vertical-link (stair / ladder) tile [`Sprite`].
///
/// Plumbing around the framework sprite (the no-bare-types framework carve-out, the same
/// justification the [`PathStepSprite`](crate::PathStepSprite) /
/// [`HoverHighlight`](crate::HoverHighlight) markers use): [`draw_vertical_links`]
/// queries `With<VerticalLinkSprite>` to find and MUTATE the pooled link sprites in place
/// rather than despawn-respawning them each frame.
#[derive(Component, Debug, Default, Clone, Copy, Eq, PartialEq, Hash)]
pub struct VerticalLinkSprite;

/// The [`TileRole`] a vertical-link endpoint maps to — the presenter-owned mapping
/// from a link's kind AND the active endpoint's direction within the link to its
/// tile-role (C2).
///
/// A [`Stair`](LinkKind::Stair) endpoint draws the direction-keyed stair role
/// (GTW-373, PROPOSED — flagged for in-engine confirm): the active endpoint is at storey
/// `active_z`, its partner endpoint at `other_z`. If the active storey is the link's
/// LOWER cell (`active_z < other_z`) you ASCEND from here, so it draws
/// [`TileRole::StairUp`]; otherwise the active storey is the UPPER
/// cell and you DESCEND, so it draws [`TileRole::StairDown`]. A
/// [`Ladder`](LinkKind::Ladder) endpoint draws the single [`TileRole::Ladder`] role
/// for either direction. The PIXELS a role resolves to are the role key's sprite def
/// (GTW-665 — [`SpriteResolveCtx`]), never a hardcoded index.
const fn link_tile_role(kind: LinkKind, active_z: i32, other_z: i32) -> TileRole {
    match kind {
        // The active storey is the link's LOWER cell -> you ascend from here (stair_up);
        // otherwise it is the UPPER cell -> you descend (stair_down).
        LinkKind::Stair { .. } if active_z < other_z => TileRole::StairUp,
        LinkKind::Stair { .. } => TileRole::StairDown,
        LinkKind::Ladder { .. } => TileRole::Ladder,
    }
}

/// `Update` ([`PresenterSystems::Scene`](crate::PresenterSystems)): draw the
/// vertical-link (stair / ladder) tiles — one cell-keyed [`Sprite`] per authored link
/// endpoint on the active storey (AC4).
///
/// Reads [`Res<VerticalLinkGraph>`](gdtf_battle_sim::vertical::VerticalLinkGraph),
/// [`Res<ActiveLevel>`](crate::ActiveLevel), and the GTW-665 [`SpriteResolveCtx`]
/// resolution bundle, then maintains a POOL of [`VerticalLinkSprite`] sprites:
///
/// - for each authored link, it draws the endpoint cell ON the active storey (the hard
///   cut, AC4) — taking (or lazily spawning) a pooled sprite, retargeting it to the
///   endpoint's role sprite def ([`link_tile_role`] → the role key's
///   `content/sprites/<key>.spritedef.ron` def; a missing def draws the LOUD magenta
///   marker — C4), moving it to [`cell_to_world_layered`] at the
///   [`Layer::VerticalLink`] band plus the def's C2 anchor offset, and showing it;
/// - every surplus pooled sprite (links not on this storey, or pooled entities beyond
///   the current set) is [`Visibility::Hidden`] — NEVER despawned (mutate, not respawn,
///   C5).
///
/// A link's two endpoints sit on adjacent storeys; iterating the GRAPH's links (not
/// [`links_from`](gdtf_battle_sim::vertical::VerticalLinkGraph::links_from)) once per link and
/// filtering each endpoint to the active storey draws exactly the on-storey endpoint —
/// `from` when its storey is active, `to` when its storey is active.
///
/// Param-only (`bevy-traps.md` #7): [`Commands`] for the lazy pool growth, the
/// [`VerticalLinkGraph`](gdtf_battle_sim::vertical::VerticalLinkGraph) / [`ActiveLevel`] /
/// [`SpriteResolveCtx`] reads, and a
/// `Query<(&mut Sprite, &mut Transform, &mut Visibility), With<VerticalLinkSprite>>` for
/// the in-place retarget + move + show/hide. Battle-gated + in
/// [`PresenterSystems::Scene`](crate::PresenterSystems) by the
/// [`TopDownRendererPlugin`](crate::TopDownRendererPlugin).
pub fn draw_vertical_links(
    mut commands: Commands,
    graph: Res<VerticalLinkGraph>,
    active: Res<ActiveLevel>,
    resolve: SpriteResolveCtx,
    mut sprites: Query<(&mut Sprite, &mut Transform, &mut Visibility), With<VerticalLinkSprite>>,
) {
    let active_level: Level = **active;
    let active_z = i32::from(*active_level);

    // The on-storey link endpoints to draw THIS update — for each authored link, the
    // endpoint cell whose z == the active storey, paired with the link kind's role.
    // Iterating the links (not `links_from`) draws each link once; the per-endpoint storey
    // filter is the AC4 hard cut (off-storey endpoints are not drawn).
    let mut to_draw: Vec<(CellLevel, TileRole)> = Vec::new();
    for link in graph.links() {
        // Each endpoint's tile is direction-keyed (GTW-373): a stair endpoint resolves to
        // stair_up / stair_down by whether the ACTIVE storey is the link's lower or upper
        // cell, so the role is computed per-endpoint with its partner's storey.
        for (endpoint, other) in [(link.from, link.to), (link.to, link.from)] {
            if endpoint.z == active_z {
                to_draw.push((endpoint, link_tile_role(link.kind, endpoint.z, other.z)));
            }
        }
    }

    // The world position of a link-tile sprite (shared by the reuse + grow paths): the
    // layered cell projection plus the resolved def's C2 anchor offset.
    let world_at = |cell: CellLevel, offset: Vec2| {
        cell_to_world_layered(cell.cell(), active_level, Layer::VerticalLink) + offset.extend(0.0)
    };
    // The shared pooled-draw walk (GTW-568): reuse the pooled link sprites in iteration
    // order (retarget + move), lazily spawn past the pool, hide the surplus — the helper
    // owns the set_if_neq visibility flips (mutate, not respawn — C5).
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

/// Lazily spawn ONE pooled vertical-link tile sprite at `world`.
///
/// The `sprite` (already retargeted to the link kind's role def) on the world
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
    use gdtf_battle_sim::vertical::LinkKind;

    use super::{TileRole, link_tile_role};

    /// GTW-373 (C4 (b)) — a STAIR endpoint picks the ASCEND role when the active storey
    /// is the link's LOWER cell and the DESCEND role when it is the UPPER cell.
    /// Pin-DISCRIMINATING: it FAILS if the up/down arms are swapped (the two roles are
    /// distinct variants; their rendered rects are pinned by the `vertical_link_draw`
    /// integration suite against the seeded defs).
    #[test]
    fn stair_endpoint_picks_up_when_lower_down_when_upper() {
        let stair = LinkKind::stair();

        // Active storey is the LOWER cell (active_z 0 < other_z 1): ASCEND -> StairUp.
        assert_eq!(
            link_tile_role(stair, 0, 1),
            TileRole::StairUp,
            "a stair endpoint on the link's LOWER cell (you ascend) must draw stair_up",
        );
        // Active storey is the UPPER cell (active_z 1 > other_z 0): DESCEND -> StairDown.
        assert_eq!(
            link_tile_role(stair, 1, 0),
            TileRole::StairDown,
            "a stair endpoint on the link's UPPER cell (you descend) must draw stair_down",
        );
    }

    /// GTW-373 (C4 (b) cont.) — a LADDER endpoint draws the single `ladder` role for
    /// EITHER direction (a ladder is not split up/down).
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
