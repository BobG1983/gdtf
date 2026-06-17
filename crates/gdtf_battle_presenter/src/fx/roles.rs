//! The DATA-DRIVEN FX-role table and its RON load/resolve chain.

use bevy::prelude::*;
use gdtf_assets::RonAsset;
use serde::Deserialize;

use crate::TileIndex;

/// The DATA-DRIVEN FX tile-role table — each one-shot FX -> its [`TileIndex`].
///
/// Loaded from the loose `assets/tiles/effect_roles.ron` through the generic
/// [`RonAsset<T>`](gdtf_assets::RonAsset) loader and resolved into a presenter-owned
/// [`EffectRoles`] resource before battle time (see [`resolve_effect_roles`]), mirroring
/// S4's `tile_roles.ron` / `load_tile_roles` / `resolve_tile_roles`. Every index is data the
/// engineer eyeballs against the effects sheet and may adjust — nothing about the index
/// choices is hardcoded in Rust; this struct only names the FX ROLES, each indexing the 16x8
/// [`SheetRole::Effects`](crate::SheetRole) sheet.
///
/// Derives [`Resource`] (the resolved runtime form), [`Deserialize`] (the authored `.ron`
/// shape), and [`TypePath`] (the bound [`RonAsset<EffectRoles>`](gdtf_assets::RonAsset)
/// requires of its payload).
#[derive(Resource, Debug, Clone, PartialEq, Eq, Deserialize, TypePath)]
pub struct EffectRoles {
    /// The [`Bleeding`](gdtf_battle_sim::Bleeding) blood/hit-flash tile (the §9 bleed-out signal).
    pub bleed:           TileIndex,
    /// The [`ArmorBroken`](gdtf_battle_sim::ArmorBroken) spark/break-burst tile (the shattered-piece signal).
    pub armor_break:     TileIndex,
    /// The [`CoverDestroyed`](gdtf_battle_sim::CoverDestroyed) debris/rubble-burst tile (the §3 cover-smashed signal).
    pub cover_destroyed: TileIndex,
}

/// The path of the loose FX-role RON, relative to the asset source root.
const EFFECT_ROLES_RON_PATH: &str = "tiles/effect_roles.ron";

/// The in-flight handle to the FX-role RON, held until it resolves into [`EffectRoles`].
///
/// A named newtype over the bevy [`Handle`] (no-bare-types: a bare handle carries no domain
/// meaning; this name says "the FX-role table being loaded"). Inserted by
/// [`load_effect_roles`] and read by [`resolve_effect_roles`], mirroring S4's
/// `TileRolesHandle`.
#[derive(Resource, Deref, Debug, Clone)]
pub struct EffectRolesHandle(pub Handle<RonAsset<EffectRoles>>);

/// `Startup`: kick off the `effect_roles.ron` load, storing its typed handle.
///
/// Loads `tiles/effect_roles.ron` as a [`RonAsset<EffectRoles>`](gdtf_assets::RonAsset)
/// through the generic GTW-136 loader and inserts the [`EffectRolesHandle`] the
/// [`resolve_effect_roles`] poll system reads. Takes `Option<Res<AssetServer>>` so a
/// `MinimalPlugins` headless app with no [`AssetServer`] no-ops rather than panicking
/// (`bevy-traps.md` #1); under `DefaultPlugins` the load fires for real.
///
/// Param-only (`bevy-traps.md` #7): [`Commands`] for the handle insert, the optional
/// [`Res<AssetServer>`] for the load.
pub fn load_effect_roles(mut commands: Commands, asset_server: Option<Res<AssetServer>>) {
    let Some(asset_server) = asset_server else {
        return;
    };
    let handle = asset_server.load::<RonAsset<EffectRoles>>(EFFECT_ROLES_RON_PATH);
    commands.insert_resource(EffectRolesHandle(handle));
}

/// `Update` (gated until [`EffectRoles`] is resolved): resolve the loaded RON into the
/// presenter-owned [`EffectRoles`] resource.
///
/// Once the [`RonAsset<EffectRoles>`](gdtf_assets::RonAsset) has settled into
/// `Assets<RonAsset<EffectRoles>>`, it clones the deserialized [`EffectRoles`] out and inserts
/// it as the resident resource so it is present before the first FX message. Run only while
/// [`EffectRolesHandle`] exists AND [`EffectRoles`] does NOT (the plugin's run-condition), so
/// it inserts once. Mirrors S4's `resolve_tile_roles`.
///
/// Param-only (`bevy-traps.md` #7): [`Commands`] for the insert, [`Res<EffectRolesHandle>`]
/// for the handle, [`Res<Assets<RonAsset<EffectRoles>>>`] for the loaded asset.
pub fn resolve_effect_roles(
    mut commands: Commands,
    handle: Res<EffectRolesHandle>,
    roles_assets: Res<Assets<RonAsset<EffectRoles>>>,
) {
    let Some(loaded) = roles_assets.get(&**handle) else {
        // Loaded-but-not-yet-in-collection (or still loading) — retry next frame; the
        // run-condition keeps this system alive until EffectRoles is resolved.
        return;
    };
    commands.insert_resource((**loaded).clone());
}
