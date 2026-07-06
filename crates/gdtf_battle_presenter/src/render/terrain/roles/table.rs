//! The atlas-index newtype, the DATA-DRIVEN terrain tile-role table, and its
//! hot-RON chain registration (the GTW-564 generic seam).

use bevy::prelude::*;
use gdtf_assets::{HotRonAppExt, HotRonChain};
use serde::{Deserialize, Serialize};

use super::vocab::TileRole;

/// An index into the terrain sheet's atlas layout — WHICH 16x16 tile a role draws.
///
/// A named newtype over `usize` (no-bare-types: an atlas index is a domain value, not
/// a bare `usize`), [`Deref`]ing to it so a consumer reads the index straight through.
/// `#[serde(transparent)]` so an authored `tile_roles.ron` field parses as a bare
/// integer (`floor: 6`), not a one-field struct. [`Serialize`] (with the same
/// transparency) lets the GTW-373 round-trip-identity test re-serialize a loaded table.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
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

impl TileRoles {
    /// Resolve a per-def terrain GRAPHIC-key string to its [`TileIndex`] in this table —
    /// the GTW-493 presenter seam (the sim spawns a
    /// [`TerrainGraphicKey`](gdtf_battle_sim::piece::TerrainGraphicKey) on every terrain entity,
    /// keyed in THIS `TileRoles` vocabulary; the presenter resolves it here).
    ///
    /// The authored `graphic_name` follows the `tile_roles.ron` vocabulary, so this
    /// classifies the key string through [`TileRole::from_key`] and reads the matching
    /// role field via [`TileRole::index_in`] — the ONE key↔role site (GTW-566 C2; the
    /// key strings themselves are spelled only in [`TileRole::as_key`]). Returns
    /// [`None`] for an unrecognized key — the caller then FALLS BACK to its
    /// presenter-owned [`TileRole`]-table default keyed on the cell's
    /// [`TerrainKind`](gdtf_battle_sim::occupancy::TerrainKind), so an out-of-vocabulary
    /// def still draws (no panic) rather than vanishing.
    ///
    /// Two `Cover` defs whose `graphic_name`s differ (e.g. `"cover"` vs `"rubble"`)
    /// therefore resolve to DISTINCT indices through this method — the per-def graphic the
    /// ticket requires, which the role-table default (keyed only on the shared
    /// [`TerrainKind::Cover`](gdtf_battle_sim::occupancy::TerrainKind)) cannot express.
    #[must_use]
    pub fn index_for_key(&self, key: &str) -> Option<TileIndex> {
        TileRole::from_key(key).map(|role| role.index_in(self))
    }
}

/// The DATA-DRIVEN terrain tile-role table — each terrain ROLE → its [`TileIndex`].
///
/// Loaded from the loose `assets/sprites/tile_roles.spritedef.ron` through the generic
/// [`RonAsset<T>`](gdtf_assets::RonAsset) loader and resolved into a presenter-owned
/// [`TileRoles`] resource before battle time (the GTW-564 generic hot-RON chain,
/// registered by [`register_tile_roles_hot_ron`]). Every index
/// is data the engineer eyeballs against the sheet and may adjust — nothing about the
/// index choices is hardcoded in Rust; this struct only names the ROLES.
///
/// Derives [`Resource`] (the resolved runtime form), [`Deserialize`] (the authored
/// `.ron` shape), [`Serialize`] (the GTW-373 round-trip-identity test re-emits a loaded
/// table), and [`TypePath`] (the bound [`RonAsset<TileRoles>`](gdtf_assets::RonAsset) requires
/// of its payload).
/// The `floor_alt_*` / `door` fields are authored for future variety; the default S4 draw
/// uses `floor` / `wall` / `cover` / `slab` / `rubble`. The Rust-side vocabulary over
/// these fields is [`TileRole`] — one variant per field (GTW-566), which the
/// vocabulary-completeness test keeps in lockstep with this serde shape.
#[derive(Resource, Debug, Clone, PartialEq, Eq, Deserialize, Serialize, TypePath)]
pub struct TileRoles {
    /// The default walkable-ground tile — in-range cells with no wall / cover / slab.
    pub floor:                TileIndex,
    /// A bolted/riveted grey-panel floor alternate (future variety).
    pub floor_alt_panel:      TileIndex,
    /// The default (NS-orientation) [`TerrainKind::Wall`](gdtf_battle_sim::occupancy::TerrainKind::Wall)
    /// tile — solid fixed geometry, the north-south-running wall strip.
    pub wall:                 TileIndex,
    /// The **east-west-running** wall tile (GTW-469) — the [`wall`](TileRoles::wall) NS sprite
    /// rotated 90° (its row-2 counterpart on the terrain sheet). A distinct AUTHORED orientation:
    /// an EW-wall [`TerrainDef`](gdtf_battle_sim::terrain::def::TerrainDef) is `sim_kind = Wall`
    /// just like the NS one (LOS / movement blocking is IDENTICAL — orientation is
    /// presentation-only) but names `graphic_name = "wall_ew"`, which resolves here so the two
    /// orientations draw perpendicular sprites. The author places the correct orientation in a
    /// prefab; orientation is never presenter-inferred.
    pub wall_ew:              TileIndex,
    /// The [`TerrainKind::Cover`](gdtf_battle_sim::occupancy::TerrainKind::Cover) tile — a chest-high cover prop.
    pub cover:                TileIndex,
    /// The VACANT weapon-emplacement tile (GTW-543) — a bolted-down heavy-gun mount a ganger
    /// ENTERS to operate, drawn distinctly from generic
    /// [`cover`](TileRoles::cover) so an emplacement reads as a manned position rather than a
    /// chest-high crate. An emplacement
    /// [`TerrainDef`](gdtf_battle_sim::terrain::def::TerrainDef) is `sim_kind = Emplacement`
    /// (cover-like blocking + a stateful enter/exit lifecycle) and names
    /// `graphic_name = "emplacement"`, which resolves here. While the emplacement is
    /// [`Occupied`](gdtf_battle_sim::emplacement::EmplacementState::Occupied) the presenter swaps its tile in
    /// place to [`emplacement_occupied`](TileRoles::emplacement_occupied) (the manned look).
    pub emplacement:          TileIndex,
    /// The OCCUPIED weapon-emplacement tile (GTW-543) — the manned variant of
    /// [`emplacement`](TileRoles::emplacement), swapped in when a ganger mans the mount
    /// ([`EmplacementState::Occupied`](gdtf_battle_sim::emplacement::EmplacementState::Occupied)) and swapped
    /// back to [`emplacement`](TileRoles::emplacement) on vacate. The presenter drives this swap in
    /// place (no despawn, the UI mutate-not-respawn rule) from a `Changed<EmplacementState>` sim
    /// signal — it is NOT reachable via a def `graphic_name` (an emplacement is always AUTHORED
    /// vacant; occupancy is a runtime state), so it is the occupied-indicator tile, not a
    /// terrain-def key.
    ///
    /// ART-REVIEW: a placeholder distinct index. A dedicated "gunner at the mount" sprite would
    /// read the occupied state more clearly — re-point this index in
    /// `assets/sprites/tile_roles.spritedef.ron` when art authors one.
    pub emplacement_occupied: TileIndex,
    /// The [`SurfaceGrid`](gdtf_battle_sim::surface::SurfaceGrid) `Present`-slab tile — a raised elevated deck.
    pub slab:                 TileIndex,
    /// The destroyed-cover / damaged tile — broken debris scatter.
    pub rubble:               TileIndex,
    /// The destroyed-**slab** tile — the engineer's-choice treatment for a smashed
    /// floor/roof slab (GTW-367 C3, mirroring how [`rubble`](TileRoles::rubble) is the
    /// destroyed-**cover** tile).
    ///
    /// CHOICE + WHY: a destroyed slab is rendered as the same broken-debris-scatter tile
    /// the rest of the terrain set uses for destruction (sharing the `rubble` atlas index
    /// `295`). A collapsed elevated deck reads as a field of rubble/debris, which keeps the
    /// destruction vocabulary consistent (smashed cover → rubble; smashed slab → rubble),
    /// and — crucially — keeps the cell still reading as *terrain* (a swap, never a hole)
    /// so the in-place mutation (no despawn) preserves the sprite. It is authored as its
    /// OWN role field (not a `rubble` alias) precisely so art can later DIVERGE it without
    /// touching the cover path.
    ///
    /// ART-REVIEW: the destroyed-slab treatment is a placeholder reuse of the rubble tile.
    /// A collapsed floor/roof slab arguably wants a DISTINCT visual — a hole punched
    /// through to the level below, a cracked/shattered deck, or a scorch — rather than the
    /// generic ground-debris scatter. Flag for art to author a dedicated destroyed-slab
    /// tile and re-point this index in `assets/sprites/tile_roles.spritedef.ron`.
    pub slab_destroyed:       TileIndex,
    /// A doorway / hatch tile (authored for future variety).
    pub door:                 TileIndex,
    /// The [`VerticalLink`](gdtf_battle_sim::vertical::VerticalLink) `Stair`-endpoint tile drawn
    /// where the active storey is the link's LOWER cell — i.e. you ASCEND from here
    /// (atlas index `29`). GTW-373 (user ruling 2026-06-23) SUPERSEDES the GTW-359 OQ-3
    /// single-`stair`-`77` constant: the stair role is split into [`stair_up`] /
    /// [`stair_down`], chosen by link direction relative to the active storey (see
    /// [`draw_vertical_links`](crate::draw_vertical_links)).
    ///
    /// [`stair_up`]: TileRoles::stair_up
    /// [`stair_down`]: TileRoles::stair_down
    pub stair_up:             TileIndex,
    /// The [`VerticalLink`](gdtf_battle_sim::vertical::VerticalLink) `Stair`-endpoint tile drawn
    /// where the active storey is the link's UPPER cell — i.e. you DESCEND from here
    /// (atlas index `28`). The down-facing companion of [`stair_up`]; same GTW-373 split
    /// of the former single `stair` role, chosen by link direction.
    ///
    /// [`stair_up`]: TileRoles::stair_up
    pub stair_down:           TileIndex,
    /// The [`VerticalLink`](gdtf_battle_sim::vertical::VerticalLink) `Ladder`-endpoint tile — a
    /// ladder cell (atlas index `235`, an UNCHANGED system constant per the user OQ-3
    /// ruling). Drawn at each ladder link cell on the active storey by the GTW-359
    /// link-cell draw (a ladder is drawn the same tile both up and down).
    pub ladder:               TileIndex,
    /// The **north-south-running** door tile (GTW-470) — a distinct AUTHORED orientation
    /// variant of the doorway hatch. A door [`TerrainDef`](gdtf_battle_sim::terrain::def::TerrainDef)
    /// is `sim_kind = Wall` carrying the sim-owned `Openable` tag (a closed door blocks like a
    /// wall; functional open/close is GTW-315) and names `graphic_name = "door_ns"`, which resolves
    /// here. The companion EW orientation is [`door_ew`](TileRoles::door_ew), the same sprite rotated
    /// 90°. The author places the correct orientation in a prefab; it is never presenter-inferred.
    pub door_ns:              TileIndex,
    /// The **east-west-running** door tile (GTW-470) — the [`door_ns`](TileRoles::door_ns) sprite
    /// rotated 90°. Same sim semantics as the NS door (`sim_kind = Wall` + `Openable`); orientation
    /// is presentation-only.
    pub door_ew:              TileIndex,
    /// The **north-south-running** stair tile you ASCEND (GTW-470) — a distinct AUTHORED
    /// orientation/direction variant. A stair [`TerrainDef`](gdtf_battle_sim::terrain::def::TerrainDef)
    /// is `sim_kind = Slab` (a walkable surface; the vertical link is GTW-388, NOT modelled here) and
    /// names `graphic_name = "stair_ns_up"`. Up and down are DISTINCT sprites (not a pure rotation);
    /// the EW companion is [`stair_ew_up`](TileRoles::stair_ew_up), the same sprite rotated 90°.
    pub stair_ns_up:          TileIndex,
    /// The **north-south-running** stair tile you DESCEND (GTW-470) — the down-direction companion
    /// of [`stair_ns_up`](TileRoles::stair_ns_up), a DISTINCT sprite. `sim_kind = Slab`; the EW
    /// companion is [`stair_ew_down`](TileRoles::stair_ew_down).
    pub stair_ns_down:        TileIndex,
    /// The **east-west-running** stair tile you ASCEND (GTW-470) — the [`stair_ns_up`](TileRoles::stair_ns_up)
    /// sprite rotated 90°. Same `sim_kind = Slab` semantics; orientation is presentation-only.
    pub stair_ew_up:          TileIndex,
    /// The **east-west-running** stair tile you DESCEND (GTW-470) — the [`stair_ns_down`](TileRoles::stair_ns_down)
    /// sprite rotated 90°. Same `sim_kind = Slab` semantics; orientation is presentation-only.
    pub stair_ew_down:        TileIndex,
}

/// The path of the loose tile-role RON, relative to the asset source root.
const TILE_ROLES_RON_PATH: &str = "sprites/tile_roles.spritedef.ron";

/// Registers the [`TileRoles`] hot-RON chain — ONE ext call onto the GTW-564
/// generic seam (kick-off / gated resolve / live redrive, keyed by the generic
/// [`HotRonHandle`](gdtf_assets::HotRonHandle)`<TileRoles>`), replacing the
/// per-site handle newtype + load/resolve/redrive triple. Self-gates on the
/// [`AssetServer`](bevy::asset::AssetServer) (`bevy-traps.md` #1), so a
/// `MinimalPlugins` headless app stays a no-op.
///
/// The live redrive overwrites [`TileRoles`] through `ResMut`, which MARKS it
/// changed — the signal `draw_static_battlefield`'s `roles.is_changed()`
/// trigger re-renders the terrain tiles on (GTW-375 C1).
pub(crate) fn register_tile_roles_hot_ron(app: &mut App) {
    app.init_hot_ron_resource::<TileRoles>(TILE_ROLES_RON_PATH);
}

/// The presenter's [`TileRoles`] chain config, exported so the EDITOR (the
/// second in-app asset host, GTW-533) can register the SAME generic redrive
/// against its own Load-side resolve — it inserts this config plus the generic
/// handle its resolve stores, and reuses
/// [`redrive_hot_ron_resource`](gdtf_assets::redrive_hot_ron_resource)`::<TileRoles, TileRoles>`
/// verbatim (no editor copy of the drain body).
#[must_use]
pub const fn tile_roles_hot_ron_chain() -> HotRonChain<TileRoles, TileRoles> {
    HotRonChain::identity(TILE_ROLES_RON_PATH)
}
