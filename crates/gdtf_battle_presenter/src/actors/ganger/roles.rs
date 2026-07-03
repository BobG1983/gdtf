//! The DATA-DRIVEN per-faction base-actor table and its hot-RON chain
//! registration (the GTW-564 generic seam).

use bevy::prelude::*;
use gdtf_assets::HotRonAppExt;
use gdtf_battle_sim::Faction;
use serde::Deserialize;

use crate::TileIndex;

/// The DATA-DRIVEN per-faction base actor table — each faction (gang) -> its base
/// [`TileIndex`] into the character sheet.
///
/// Loaded from the loose `assets/sprites/character_roles.spritedef.ron` through the generic
/// [`RonAsset<T>`](gdtf_assets::RonAsset) loader and resolved into a presenter-owned
/// [`CharacterRoles`] resource before battle time (the GTW-564 generic hot-RON
/// chain, registered by [`register_character_roles_hot_ron`]), exactly mirroring the
/// S4 `tile_roles.ron` chain. Each faction's actor is a contiguous run of 4 cells in the sheet; the drawn
/// index is `base + facing_frame` ([`facing_frame`](super::facing_frame)). Every index
/// is data the engineer eyeballs against the sheet and may adjust — nothing about the
/// index choices is hardcoded in Rust; this struct only names the FACTIONS.
///
/// Derives [`Resource`] (the resolved runtime form), [`Deserialize`] (the authored
/// `.ron` shape), and [`TypePath`] (the bound [`RonAsset<CharacterRoles>`](gdtf_assets::RonAsset) requires of
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
const CHARACTER_ROLES_RON_PATH: &str = "sprites/character_roles.spritedef.ron";

/// Registers the [`CharacterRoles`] hot-RON chain — ONE ext call onto the
/// GTW-564 generic seam (kick-off / gated resolve / live redrive, keyed by the
/// generic [`HotRonHandle`](gdtf_assets::HotRonHandle)`<CharacterRoles>`),
/// replacing the per-site handle newtype + load/resolve/redrive triple.
/// Self-gates on the [`AssetServer`](bevy::asset::AssetServer)
/// (`bevy-traps.md` #1), so a `MinimalPlugins` headless app stays a no-op.
///
/// The live redrive overwrites [`CharacterRoles`] through `ResMut`, which MARKS
/// it changed — the signal
/// [`reindex_ganger_sprites_on_character_roles_change`](super::reindex_ganger_sprites_on_character_roles_change)
/// re-indexes the persistent ganger sprites on (GTW-375 C3).
pub(crate) fn register_character_roles_hot_ron(app: &mut App) {
    app.init_hot_ron_resource::<CharacterRoles>(CHARACTER_ROLES_RON_PATH);
}
