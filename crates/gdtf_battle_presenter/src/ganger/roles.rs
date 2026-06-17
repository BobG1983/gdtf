//! The DATA-DRIVEN per-faction base-actor table and its RON load/resolve chain.

use bevy::prelude::*;
use gdtf_assets::RonAsset;
use gdtf_battle_sim::Faction;
use serde::Deserialize;

use crate::TileIndex;

/// The DATA-DRIVEN per-faction base actor table — each faction (gang) -> its base
/// [`TileIndex`] into the character sheet.
///
/// Loaded from the loose `assets/tiles/character_roles.ron` through the generic
/// [`RonAsset<T>`](gdtf_assets::RonAsset) loader and resolved into a presenter-owned
/// [`CharacterRoles`] resource before battle time (see [`resolve_character_roles`]),
/// exactly mirroring the S4 `tile_roles.ron` / `load_tile_roles` / `resolve_tile_roles`
/// chain. Each faction's actor is a contiguous run of 4 cells in the sheet; the drawn
/// index is `base + facing_frame` ([`facing_frame`](super::facing_frame)). Every index
/// is data the engineer eyeballs against the sheet and may adjust — nothing about the
/// index choices is hardcoded in Rust; this struct only names the FACTIONS.
///
/// Derives [`Resource`] (the resolved runtime form), [`Deserialize`] (the authored
/// `.ron` shape), and [`TypePath`] (the bound [`RonAsset<CharacterRoles>`] requires of
/// its payload). At minimum faction 0 and faction 1 map to two visibly distinct actor
/// base tiles; a faction with no authored base falls back to [`Self::faction_0`] (the
/// only design factions today are 0 and 1, the two gangs).
#[derive(Resource, Debug, Clone, PartialEq, Eq, Deserialize, TypePath)]
pub struct CharacterRoles {
    /// Faction (gang) 0's base actor tile index — the first gang's 4-frame actor.
    pub faction_0: TileIndex,
    /// Faction (gang) 1's base actor tile index — the opposing gang's 4-frame actor,
    /// authored to a visibly distinct actor from [`Self::faction_0`].
    pub faction_1: TileIndex,
}

impl CharacterRoles {
    /// The base actor [`TileIndex`] for `faction`, read from this resource.
    ///
    /// Faction 0 -> [`Self::faction_0`], faction 1 -> [`Self::faction_1`]. Any other
    /// gang index (none exist in the current two-gang design) falls back to
    /// [`Self::faction_0`] rather than panicking on an out-of-table faction.
    ///
    /// Not `const`: reading the gang index derefs [`Faction`]'s derived [`Deref`], which
    /// is not a `const` impl (the `f32`-newtype / `Deref`-blocks-const idiom).
    #[must_use]
    pub fn base_for(&self, faction: Faction) -> TileIndex {
        match *faction {
            1 => self.faction_1,
            _ => self.faction_0,
        }
    }
}

/// The path of the loose character-role RON, relative to the asset source root.
const CHARACTER_ROLES_RON_PATH: &str = "tiles/character_roles.ron";

/// The in-flight handle to the character-role RON, held until it resolves into
/// [`CharacterRoles`].
///
/// A named newtype over the bevy [`Handle`] (no-bare-types: a bare handle carries no
/// domain meaning; this name says "the character-role table being loaded"). Inserted by
/// [`load_character_roles`] and read by [`resolve_character_roles`] — the S4
/// `TileRolesHandle` precedent for the character table.
#[derive(Resource, Deref, Debug, Clone)]
pub struct CharacterRolesHandle(pub Handle<RonAsset<CharacterRoles>>);

/// `Startup`: kick off the `character_roles.ron` load, storing its typed handle.
///
/// Loads `tiles/character_roles.ron` as a
/// [`RonAsset<CharacterRoles>`](gdtf_assets::RonAsset) through the generic GTW-136 loader
/// and inserts the [`CharacterRolesHandle`] the [`resolve_character_roles`] poll system
/// reads — the S4 [`load_tile_roles`](crate::load_tile_roles) precedent. Takes
/// `Option<Res<AssetServer>>` so a `MinimalPlugins` headless app with no [`AssetServer`]
/// no-ops rather than panicking (`bevy-traps.md` #1); under `DefaultPlugins` (the app +
/// the `AssetServer` harness) the load fires for real and the resolve runs.
///
/// Param-only (`bevy-traps.md` #7): [`Commands`] for the handle insert, the optional
/// [`Res<AssetServer>`] for the load.
pub fn load_character_roles(mut commands: Commands, asset_server: Option<Res<AssetServer>>) {
    let Some(asset_server) = asset_server else {
        return;
    };
    let handle = asset_server.load::<RonAsset<CharacterRoles>>(CHARACTER_ROLES_RON_PATH);
    commands.insert_resource(CharacterRolesHandle(handle));
}

/// `Update` (gated until [`CharacterRoles`] is resolved): resolve the loaded RON into
/// the presenter-owned [`CharacterRoles`] resource.
///
/// Once the [`RonAsset<CharacterRoles>`](gdtf_assets::RonAsset) has settled into
/// `Assets<RonAsset<CharacterRoles>>` (a transient one-frame "loaded but not yet in the
/// collection" state simply leaves it un-inserted this pass — retried next frame), it
/// clones the deserialized [`CharacterRoles`] out and inserts it as the resident
/// resource so it is present before the first ganger draws. Run only while
/// [`CharacterRolesHandle`] exists AND [`CharacterRoles`] does NOT (the plugin's
/// run-condition), so it inserts once — the S4 [`resolve_tile_roles`](crate::resolve_tile_roles)
/// precedent.
///
/// Param-only (`bevy-traps.md` #7): [`Commands`] for the insert,
/// [`Res<CharacterRolesHandle>`] for the handle,
/// [`Res<Assets<RonAsset<CharacterRoles>>>`] for the loaded asset.
pub fn resolve_character_roles(
    mut commands: Commands,
    handle: Res<CharacterRolesHandle>,
    roles_assets: Res<Assets<RonAsset<CharacterRoles>>>,
) {
    let Some(loaded) = roles_assets.get(&**handle) else {
        // Loaded-but-not-yet-in-collection (or still loading) — retry next frame; the
        // run-condition keeps this system alive until CharacterRoles is resolved.
        return;
    };
    commands.insert_resource((**loaded).clone());
}
