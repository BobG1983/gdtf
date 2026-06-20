//! The atlas-index newtype, the DATA-DRIVEN terrain tile-role table, and its RON
//! load/resolve chain.

use bevy::prelude::*;
use gdtf_assets::RonAsset;
use serde::Deserialize;

/// An index into the terrain sheet's atlas layout — WHICH 16x16 tile a role draws.
///
/// A named newtype over `usize` (no-bare-types: an atlas index is a domain value, not
/// a bare `usize`), [`Deref`]ing to it so a consumer reads the index straight through.
/// `#[serde(transparent)]` so an authored `tile_roles.ron` field parses as a bare
/// integer (`floor: 6`), not a one-field struct.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(transparent)]
pub struct TileIndex(usize);

impl TileIndex {
    /// Build a tile index from its layout position.
    ///
    /// `usize` is the index space of the atlas layout's `textures` collection (a
    /// collection-index, the no-bare-types framework-plumbing carve-out for the inner
    /// value), wrapped here as the named domain [`TileIndex`].
    #[must_use]
    pub const fn new(index: usize) -> Self {
        Self(index)
    }
}

/// The DATA-DRIVEN terrain tile-role table — each terrain ROLE → its [`TileIndex`].
///
/// Loaded from the loose `assets/tiles/tile_roles.ron` through the generic
/// [`RonAsset<T>`](gdtf_assets::RonAsset) loader and resolved into a presenter-owned
/// [`TileRoles`] resource before battle time (see [`resolve_tile_roles`]). Every index
/// is data the engineer eyeballs against the sheet and may adjust — nothing about the
/// index choices is hardcoded in Rust; this struct only names the ROLES.
///
/// Derives [`Resource`] (the resolved runtime form), [`Deserialize`] (the authored
/// `.ron` shape), and [`TypePath`] (the bound [`RonAsset<TileRoles>`] requires of its
/// payload). The `floor_alt_*` / `door` fields are authored for future variety; the
/// default S4 draw uses `floor` / `wall` / `cover` / `slab` / `rubble`.
#[derive(Resource, Debug, Clone, PartialEq, Eq, Deserialize, TypePath)]
pub struct TileRoles {
    /// The default walkable-ground tile — in-range cells with no wall / cover / slab.
    pub floor:           TileIndex,
    /// A bolted/riveted grey-panel floor alternate (future variety).
    pub floor_alt_panel: TileIndex,
    /// A flat blue-grey stone floor alternate (future variety).
    pub floor_alt_stone: TileIndex,
    /// A flat orange/tan dirt floor alternate (future variety).
    pub floor_alt_dirt:  TileIndex,
    /// A green grass-field floor alternate (future variety).
    pub floor_alt_grass: TileIndex,
    /// The [`TerrainKind::Wall`](gdtf_battle_sim::TerrainKind::Wall) tile — solid fixed geometry.
    pub wall:            TileIndex,
    /// The [`TerrainKind::Cover`](gdtf_battle_sim::TerrainKind::Cover) tile — a chest-high cover prop.
    pub cover:           TileIndex,
    /// The [`SurfaceGrid`](gdtf_battle_sim::SurfaceGrid) `Present`-slab tile — a raised elevated deck.
    pub slab:            TileIndex,
    /// The destroyed-cover / damaged tile — broken debris scatter.
    pub rubble:          TileIndex,
    /// A doorway / hatch tile (authored for future variety).
    pub door:            TileIndex,
}

/// The path of the loose tile-role RON, relative to the asset source root.
const TILE_ROLES_RON_PATH: &str = "tiles/tile_roles.ron";

/// The in-flight handle to the tile-role RON, held until it resolves into [`TileRoles`].
///
/// A named newtype over the bevy [`Handle`] (no-bare-types: a bare handle carries no
/// domain meaning; this name says "the tile-role table being loaded"). Inserted by
/// [`load_tile_roles`] and read by [`resolve_tile_roles`].
#[derive(Resource, Deref, Debug, Clone)]
pub struct TileRolesHandle(Handle<RonAsset<TileRoles>>);

impl TileRolesHandle {
    /// Wrap the in-flight tile-role RON handle.
    #[must_use]
    pub const fn new(handle: Handle<RonAsset<TileRoles>>) -> Self {
        Self(handle)
    }
}

/// `Startup`: kick off the `tile_roles.ron` load, storing its typed handle.
///
/// Loads `tiles/tile_roles.ron` as a [`RonAsset<TileRoles>`](gdtf_assets::RonAsset)
/// through the generic GTW-136 loader and inserts the [`TileRolesHandle`] the
/// [`resolve_tile_roles`] poll system reads. Takes `Option<Res<AssetServer>>` so a
/// `MinimalPlugins` headless app with no [`AssetServer`] no-ops rather than panicking
/// (`bevy-traps.md` #1); under `DefaultPlugins` (the app + the `AssetServer` harness) the
/// load fires for real and the resolve runs.
///
/// Param-only (`bevy-traps.md` #7): [`Commands`] for the handle insert, the optional
/// [`Res<AssetServer>`] for the load.
pub fn load_tile_roles(mut commands: Commands, asset_server: Option<Res<AssetServer>>) {
    let Some(asset_server) = asset_server else {
        return;
    };
    let handle = asset_server.load::<RonAsset<TileRoles>>(TILE_ROLES_RON_PATH);
    commands.insert_resource(TileRolesHandle::new(handle));
}

/// `Update` (gated until [`TileRoles`] is resolved): resolve the loaded RON into the
/// presenter-owned [`TileRoles`] resource.
///
/// Once the [`RonAsset<TileRoles>`](gdtf_assets::RonAsset) has settled into
/// `Assets<RonAsset<TileRoles>>` (a transient one-frame "loaded but not yet in the
/// collection" state simply leaves it un-inserted this pass — retried next frame), it
/// clones the deserialized [`TileRoles`] out and inserts it as the resident resource so
/// it is present before the first `BattleReady`. Run only while [`TileRolesHandle`]
/// exists AND [`TileRoles`] does NOT (the plugin's run-condition), so it inserts once.
///
/// Param-only (`bevy-traps.md` #7): [`Commands`] for the insert, [`Res<TileRolesHandle>`]
/// for the handle, [`Res<Assets<RonAsset<TileRoles>>>`] for the loaded asset.
pub fn resolve_tile_roles(
    mut commands: Commands,
    handle: Res<TileRolesHandle>,
    roles_assets: Res<Assets<RonAsset<TileRoles>>>,
) {
    let Some(loaded) = roles_assets.get(&**handle) else {
        // Loaded-but-not-yet-in-collection (or still loading) — retry next frame; the
        // run-condition keeps this system alive until TileRoles is resolved.
        return;
    };
    commands.insert_resource((**loaded).clone());
}
