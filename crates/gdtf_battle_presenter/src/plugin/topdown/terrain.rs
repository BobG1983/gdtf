//! Terrain draw registration: the static-battlefield draw, the destruction /
//! emplacement swap reactions, and the vertical-link (stair / ladder) draw.

use bevy::{ecs::message::Messages, image::Image, prelude::*};
use gdtf_battle_sim::{
    cover::CoverLedger,
    occupancy_sync::SlabDestroyed,
    prelude::{BattleInProgress, OccupancyGrid},
    surface::SurfaceGrid,
    vertical::VerticalLinkGraph,
};
use gdtf_content_families::sprites::SpriteDefRegistry;

use crate::{
    MissingTileTexture, PresenterSystems, draw_static_battlefield, draw_vertical_links,
    indicate_emplacement_occupied, render::terrain::setup_missing_tile_texture,
    restamp_tiles_on_def_change, swap_destroyed_cover, swap_destroyed_slab,
};

/// GTW-218 (S4) adds the static terrain draw here (re-plumbed by GTW-665 onto the
/// sprite-def resolution): it registers the one-shot [`draw_static_battlefield`] in the
/// [`PresenterSystems::Scene`] stage (GTW-623 — the drawn-world stage; fog and the
/// overlays order after it by STAGE MEMBERSHIP), gated
/// `run_if(resource_exists::<BattleInProgress>)` (the sim's battle-in-progress witness, so
/// the draw runs only DURING a live battle), plus the `Startup`
/// [`setup_missing_tile_texture`] that mints the C4 magenta marker (gated on
/// [`Assets<Image>`] existing so a `MinimalPlugins` app no-ops — `bevy-traps.md` #1).
///
/// The draw is gated on `BattleInProgress` AND on every resource it READS existing: a
/// battle can be `BattleInProgress` while the [`SpriteDefRegistry`] / [`AssetServer`] /
/// [`MissingTileTexture`] are absent (a `MinimalPlugins` headless app with no asset
/// stack never loads/mints them; the registry is published by the HOST's
/// `register_content_family::<SpriteDefsFamily>` — the game's / editor's Load pass), so
/// without those extra guards the system would fail param validation when a resource is
/// missing — the exact panic `bevy-traps.md` #1 (and the ticket's "a no-resource state
/// must NOT panic the draw") demands we gate. The draw scans the three sim grids + the
/// resolution bundle.
pub(super) fn register_terrain_draw(app: &mut App) {
    app.add_systems(
        Startup,
        setup_missing_tile_texture.run_if(resource_exists::<Assets<Image>>),
    );
    app.add_systems(
        Update,
        draw_static_battlefield
            .in_set(PresenterSystems::Scene)
            .run_if(
                resource_exists::<BattleInProgress>
                    .and_then(resource_exists::<SpriteDefRegistry>)
                    .and_then(resource_exists::<AssetServer>)
                    .and_then(resource_exists::<MissingTileTexture>)
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
/// despawn — the UI mutate-not-respawn rule, C7) — plus the GTW-666
/// [`restamp_tiles_on_def_change`] hot-reload restamp, which rides the same stage + gates and
/// is ordered explicitly after the draw + all three swap writers (`bevy-traps.md` #3: five
/// systems touch `Assets<TerrainFogMaterial>`, so the order is pinned). Extracted from
/// `build` to keep it under the `too_many_lines` lint (mirrors
/// [`register_fx_flash_systems`](super::fx::register_fx_flash_systems) /
/// [`register_fog_systems`](super::fog::register_fog_systems)).
///
/// Each reaction is gated `run_if(resource_exists::<BattleInProgress>)` (the live-battle witness)
/// AND on the GTW-665 resolution bundle's resources ([`SpriteDefRegistry`] +
/// [`AssetServer`] + [`MissingTileTexture`] — read to retarget the destroyed tile). The
/// slab reaction's
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
            .run_if(resource_exists::<BattleInProgress>.and_then(sprite_resolution_ready)),
    )
    .add_systems(
        Update,
        swap_destroyed_slab.in_set(PresenterSystems::Scene).run_if(
            resource_exists::<BattleInProgress>
                .and_then(sprite_resolution_ready)
                .and_then(resource_exists::<Messages<SlabDestroyed>>),
        ),
    )
    // GTW-543: the weapon-emplacement OCCUPIED-state visual indicator — the state analogue of
    // the two destruction swaps. Rather than a one-shot destruction MESSAGE it reacts to the
    // sim's per-entity `Changed<EmplacementState>` (the enter/exit toggle flips it) and swaps
    // the drawn tile's material in place between the VACANT `emplacement` tile and the
    // OCCUPIED `emplacement_occupied` tile — mutate-not-respawn, the same in-place material
    // retarget the destruction swaps do. It reads no message buffer (a `Changed` query, not a
    // `MessageReader`), so it is gated on `BattleInProgress` (the live-battle witness) AND the
    // resolution bundle — the swap_destroyed_cover gate.
    .add_systems(
        Update,
        indicate_emplacement_occupied
            .in_set(PresenterSystems::Scene)
            .run_if(resource_exists::<BattleInProgress>.and_then(sprite_resolution_ready)),
    )
    // GTW-666: the sprite-def hot-reload RESTAMP — when the SpriteDefRegistry changes (the
    // family redrive rebuilt it from a re-saved `.spritedef.ron`), every already-drawn tile
    // re-resolves its StampedGraphic key and re-applies texture/rect/anchor IN PLACE through
    // the swaps' tick-quiet write path. Same Scene stage + gates as the swaps; ordered
    // explicitly AFTER the draw and the three swap writers (bevy-traps #3 — all five touch
    // Assets<TerrainFogMaterial>, so the shared-data order must be pinned, not ambient): on a
    // frame where a swap retargets AND the registry changed, the restamp re-resolves the
    // final (post-swap) stamp against the new defs.
    .add_systems(
        Update,
        restamp_tiles_on_def_change
            .in_set(PresenterSystems::Scene)
            .after(draw_static_battlefield)
            .after(swap_destroyed_cover)
            .after(swap_destroyed_slab)
            .after(indicate_emplacement_occupied)
            .run_if(resource_exists::<BattleInProgress>.and_then(sprite_resolution_ready)),
    );
}

/// The shared run CONDITION for every system taking the GTW-665
/// [`SpriteResolveCtx`](crate::SpriteResolveCtx) bundle: the [`SpriteDefRegistry`] (the
/// HOST's Load pass publishes it), the [`AssetServer`] (a `MinimalPlugins` app has none),
/// and the [`MissingTileTexture`] (minted at `Startup` only with an image-asset stack)
/// must ALL exist. `Assets<Image>` rides the `AssetServer` (the asset stack registers
/// both), so it needs no separate arm. A plain `Option<Res<…>>` condition system (the
/// `resource_exists` shape), composed via `.and_then(sprite_resolution_ready)`.
const fn sprite_resolution_ready(
    defs: Option<Res<SpriteDefRegistry>>,
    asset_server: Option<Res<AssetServer>>,
    missing: Option<Res<MissingTileTexture>>,
) -> bool {
    defs.is_some() && asset_server.is_some() && missing.is_some()
}

/// GTW-359 (AC4 / C2) + GTW-373: the vertical-link (stair / ladder) cell draw. It
/// reads the sim's `VerticalLinkGraph` + the GTW-665 sprite-def resolution bundle and
/// draws one direction-keyed stair / ladder
/// tile per authored link endpoint on the active storey (the hard cut),
/// pooled + mutated in place (C5). Gated on
/// `BattleInProgress` (the live-battle witness) AND on every resource it reads:
/// `VerticalLinkGraph` (inserted by the sim's `setup_battle`, absent in a focused
/// harness that opens `BattleInProgress` directly) plus the resolution bundle
/// (a `MinimalPlugins` headless app with no asset stack never has it)
/// — so a no-resource state simply does not draw rather than panicking param
/// validation (`bevy-traps.md` #1).
pub(super) fn register_vertical_links(app: &mut App) {
    app.add_systems(
        Update,
        draw_vertical_links.in_set(PresenterSystems::Scene).run_if(
            resource_exists::<BattleInProgress>
                .and_then(resource_exists::<VerticalLinkGraph>)
                .and_then(sprite_resolution_ready),
        ),
    );
}
