//! Terrain draw registration: the static-battlefield draw, the destruction /
//! emplacement swap reactions, and the vertical-link (stair / ladder) draw.

use bevy::{ecs::message::Messages, prelude::*};
use gdtf_battle_sim::{
    cover::CoverLedger,
    occupancy_sync::SlabDestroyed,
    prelude::{BattleInProgress, OccupancyGrid},
    surface::SurfaceGrid,
    vertical::VerticalLinkGraph,
};

use crate::{
    PresenterSystems, TileRoles, TopDownAtlases, draw_static_battlefield, draw_vertical_links,
    indicate_emplacement_occupied, swap_destroyed_cover, swap_destroyed_slab,
};

/// GTW-218 (S4) adds the static terrain draw here: it registers the [`TileRoles`]
/// hot-RON chain through the GTW-564 generic seam (self-gated on an
/// [`AssetServer`] so a `MinimalPlugins` app no-ops rather than panicking on the asset
/// registration), inserts the [`ActiveLevel`](crate::ActiveLevel) default (level 0), and
/// registers the one-shot [`draw_static_battlefield`] + the [`swap_destroyed_cover`] /
/// [`swap_destroyed_slab`] destruction reactions (GTW-367) in the
/// [`PresenterSystems::Scene`] stage (GTW-623 — the drawn-world stage; fog and the
/// overlays order after it by STAGE MEMBERSHIP), all gated
/// `run_if(resource_exists::<BattleInProgress>)` (the sim's battle-in-progress witness, so
/// the draw runs only DURING a live battle).
///
/// Both draw systems are gated `run_if(resource_exists::<BattleInProgress>)` (the
/// contract's battle gate) AND on the resources they READ existing: a battle can
/// be `BattleInProgress` while the renderer's `TileRoles` / `TopDownAtlases` are
/// absent (a `MinimalPlugins` headless app with no `AssetServer` never loads
/// them), so without those extra guards the systems would fail param validation
/// when the resource is missing — the exact panic `bevy-traps.md` #1 (and the
/// ticket's "a no-resource state must NOT panic the draw") demands we gate. The
/// draw scans the three sim grids + the two render resources; the swap reaction
/// needs only `TileRoles` (and the always-present `ActiveLevel`).
pub(super) fn register_terrain_draw(app: &mut App) {
    app.add_systems(
        Update,
        draw_static_battlefield
            .in_set(PresenterSystems::Scene)
            .run_if(
                resource_exists::<BattleInProgress>
                    .and_then(resource_exists::<TileRoles>)
                    .and_then(resource_exists::<TopDownAtlases>)
                    .and_then(resource_exists::<OccupancyGrid>)
                    .and_then(resource_exists::<CoverLedger>)
                    .and_then(resource_exists::<SurfaceGrid>),
            ),
    );
}

/// Registers the GTW-367 terrain destruction-swap reactions: the S4 [`swap_destroyed_cover`]
/// (a [`CoverDestroyed`](gdtf_battle_sim::occupancy_sync::CoverDestroyed) swaps the cell's terrain sprite to the
/// `rubble` tile) and the GTW-367 [`swap_destroyed_slab`] (a
/// [`SlabDestroyed`](gdtf_battle_sim::occupancy_sync::SlabDestroyed) swaps it to the `slab_destroyed` tile) — both
/// in the [`PresenterSystems::Scene`] stage (GTW-623 — a state swap is part of the drawn
/// world the fog modulates), MUTATING the existing tile's material in place (no
/// despawn — the UI mutate-not-respawn rule, C7). Extracted from `build` to keep it under the
/// `too_many_lines` lint (mirrors
/// [`register_fx_flash_systems`](super::fx::register_fx_flash_systems) /
/// [`register_fog_systems`](super::fog::register_fog_systems)).
///
/// Each reaction is gated `run_if(resource_exists::<BattleInProgress>)` (the live-battle witness)
/// AND `resource_exists::<TileRoles>` (read for the destroyed tile index). The slab reaction's
/// [`MessageReader<SlabDestroyed>`](bevy::ecs::message::MessageReader) panics param validation
/// without its `Messages<SlabDestroyed>` buffer (`bevy-traps.md` #4), so it is ALSO gated on
/// that buffer existing — a REAL gate (GTW-623 C4): the presenter no longer `add_message`s
/// the sim-owned `SlabDestroyed` buffer itself. The sim's `BattleSimPlugin` registers it in a
/// real battle; a focused harness that opens `BattleInProgress` directly and wants the swap
/// seeds the buffer itself (the `terrain_draw` harness does), and one that omits it simply
/// keeps the reaction inert instead of failing validation.
pub(super) fn register_destruction_swaps(app: &mut App) {
    app.add_systems(
        Update,
        swap_destroyed_cover
            .in_set(PresenterSystems::Scene)
            .run_if(resource_exists::<BattleInProgress>.and_then(resource_exists::<TileRoles>)),
    )
    .add_systems(
        Update,
        swap_destroyed_slab.in_set(PresenterSystems::Scene).run_if(
            resource_exists::<BattleInProgress>
                .and_then(resource_exists::<TileRoles>)
                .and_then(resource_exists::<Messages<SlabDestroyed>>),
        ),
    )
    // GTW-543: the weapon-emplacement OCCUPIED-state visual indicator — the state analogue of
    // the two destruction swaps. Rather than a one-shot destruction MESSAGE it reacts to the
    // sim's per-entity `Changed<EmplacementState>` (the enter/exit toggle flips it) and swaps
    // the drawn tile's material in place between the VACANT `emplacement` tile and the
    // OCCUPIED `emplacement_occupied` tile — mutate-not-respawn, the same in-place material
    // re-index the destruction swaps do. It reads no message buffer (a `Changed` query, not a
    // `MessageReader`), so it is gated on `BattleInProgress` (the live-battle witness) AND
    // `TileRoles` (read for the two emplacement tile indices) — the swap_destroyed_cover gate.
    .add_systems(
        Update,
        indicate_emplacement_occupied
            .in_set(PresenterSystems::Scene)
            .run_if(resource_exists::<BattleInProgress>.and_then(resource_exists::<TileRoles>)),
    );
}

/// GTW-359 (AC4 / C2) + GTW-373: the vertical-link (stair / ladder) cell draw. It
/// reads the sim's `VerticalLinkGraph` + the presenter's `TileRoles` /
/// `TopDownAtlases` and draws one direction-keyed stair (up 29 / down 28) / ladder
/// (235) tile per authored link endpoint on the active storey (the hard cut),
/// pooled + mutated in place (C5). Gated on
/// `BattleInProgress` (the live-battle witness) AND on every resource it reads:
/// `VerticalLinkGraph` (inserted by the sim's `setup_battle`, absent in a focused
/// harness that opens `BattleInProgress` directly), `TileRoles`, and `TopDownAtlases`
/// (a `MinimalPlugins` headless app with no `AssetServer` never loads the latter two)
/// — so a no-resource state simply does not draw rather than panicking param
/// validation (`bevy-traps.md` #1).
pub(super) fn register_vertical_links(app: &mut App) {
    app.add_systems(
        Update,
        draw_vertical_links.in_set(PresenterSystems::Scene).run_if(
            resource_exists::<BattleInProgress>
                .and_then(resource_exists::<VerticalLinkGraph>)
                .and_then(resource_exists::<TileRoles>)
                .and_then(resource_exists::<TopDownAtlases>),
        ),
    );
}
