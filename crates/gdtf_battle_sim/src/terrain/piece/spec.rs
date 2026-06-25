//! The **authoring spec** — the `TerrainSpec` a `assets/terrain/*.terrain.ron`
//! deserializes into (GTW-394), the terrain mirror of
//! [`WeaponSpec`](crate::weapon::WeaponSpec) / [`ArmorSpec`](crate::armor::ArmorSpec).

use bevy::reflect::TypePath;
use serde::Deserialize;

use super::components::{FootfallSound, TerrainGraphicKey};
use crate::{
    armor::{ArmorHardness, ArmorProtection},
    cover::{CoverHp, HeightBand},
    slab::SlabHp,
    tuning::MoveCost,
};

/// The **authoring struct** a `assets/terrain/*.terrain.ron` deserializes into —
/// every terrain piece stat the sim model carries, MINUS the
/// [`TerrainName`](super::TerrainName) (the name is the FILE KEY, supplied by the
/// loader from the file's stem, exactly as [`WeaponSpec`](crate::weapon::WeaponSpec)
/// and [`ArmorSpec`](crate::armor::ArmorSpec) drop the name).
///
/// A common shared header (the presentation hooks carried by ALL piece types) plus a
/// `kind: TerrainKindSpec` payload whose variants carry the kind-specific stats. This
/// mirrors how [`WeaponSpec`](crate::weapon::WeaponSpec) varies behaviour through a
/// `fire_mode` sub-enum list while keeping the shared stats flat — and is the only
/// honest model for the five piece kinds (FLOOR / WALL / COVER / SCATTER / SLAB) that
/// differ structurally (a floor has a move cost but no HP; a wall/cover/scatter has HP
/// but no move cost; a slab has HP but no height band).
///
/// Every field is an existing or new named newtype (no-bare-types); the authored
/// magnitudes are tuning DATA (commented in the `.ron`), NOT pinned by tests. Derives
/// [`Deserialize`] so the loose `.ron` parses, and [`TypePath`] because
/// `RonAsset<TerrainSpec>` requires its payload to be [`TypePath`] (the same bound
/// [`WeaponSpec`](crate::weapon::WeaponSpec) /
/// [`ArmorSpec`](crate::armor::ArmorSpec) satisfy).
///
/// **Not `Copy`** — the [`TerrainGraphicKey`] / [`FootfallSound`] fields own
/// `String`s; it is `Clone` so the registry can hold specs by value.
///
/// **`TerrainSpec` and [`TerrainKindSpec`] carry NO `Default`** — they are RON
/// payloads (like `WeaponSpec`/`ArmorSpec`, which carry no `Default`), not
/// `bsn!`-spawned components that require a `Default` sentinel (see memory:
/// *bsn-sentinel-Defaults*).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, TypePath)]
pub struct TerrainSpec {
    /// The presentation graphic role key — an opaque string the presenter resolves
    /// via `TileRoles` to a tile atlas entry. The sim stores it render-free and
    /// never resolves it to an atlas index (the presenter's `TileRoles` does).
    pub graphic:  TerrainGraphicKey,
    /// The footfall sound asset key — an opaque string the presenter resolves to
    /// an audio clip. No audio system is built yet; carried for the future
    /// footfall-audio pass without a schema change.
    pub footfall: FootfallSound,
    /// The kind-specific payload — which of the five piece kinds this is and its
    /// associated stats (floor move cost vs. structural HP/armor/band).
    pub kind:     TerrainKindSpec,
}

/// The **kind enum** — which of the five terrain piece kinds a [`TerrainSpec`]
/// describes, carrying the kind-specific stats.
///
/// Five variants = the five C1 piece types exactly (FLOOR / WALL / COVER / SCATTER /
/// SLAB). Three of them (Wall / Cover / Scatter) share [`StructuralSpec`] (HP +
/// armor + band — the wall-or-prop "one shape for both" precedent in
/// [`CoverEntry`](crate::cover::CoverEntry)); Slab gets its own payload because a
/// slab carries NO height band (canon: [`SlabEntry`](crate::slab::SlabEntry) has no
/// band — "a slab spans the whole z-boundary, so there is no band to clear").
///
/// **Deviation flag for `/gate`:** the ticket wording "height band for
/// destructible/cover/slab" could be read as slab-needs-a-band. The sim canon
/// ([`SlabEntry`](crate::slab::SlabEntry), `battle-space.md`, `resolution.md §2`) is
/// unambiguous that a slab has no band. `SlabPieceSpec` omits the band per canon;
/// the gate should verify this against the ticket.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize)]
pub enum TerrainKindSpec {
    /// Walkable ground — carries a terrain move cost, no destructible stats.
    /// A ganger steps onto this; it has no HP (you do not shoot the floor).
    Floor(FloorSpec),
    /// Solid blocking geometry — a destructible structural piece. A wall fills
    /// the cell; nothing flies over it within a storey.
    Wall(StructuralSpec),
    /// Chest-high cover prop — a destructible structural piece a round may clear
    /// (resolution.md §3: a round with a high enough band clears LOW cover).
    Cover(StructuralSpec),
    /// Loose scatter / debris prop — a destructible structural piece (low, fragile).
    /// Shares the wall/cover structural model: it is a prop that can be shot apart.
    Scatter(StructuralSpec),
    /// Floor/roof slab — a destructible structural piece spanning a z-boundary.
    /// A slab carries **no** height band: unlike cover (which a round must fly
    /// higher than to clear), a slab spans the whole z-boundary, so the march
    /// stops on an intact slab regardless of band (`resolution.md §2`).
    Slab(SlabPieceSpec),
}

/// A **floor piece's stats** — only a terrain move cost.
///
/// A floor carries no HP or armor (you do not shoot the floor you stand on). The
/// move cost is a per-floor-type value (the richer per-floor-type movement cost that
/// [`MoveCosts`](crate::tuning::MoveCosts) flags as "a follow-up"). Downstream
/// consumption (binding a generated cell's `TerrainName` → its `move_cost`) is a
/// future epic ticket; the registry is dormant after GTW-394 (the data-carrier /
/// schema + loader slice).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
pub struct FloorSpec {
    /// The TU cost to step onto this floor — the per-floor-type movement cost
    /// (REUSE [`MoveCost`] from `crate::tuning`; a starting point, tunable).
    pub move_cost: MoveCost,
}

/// A **wall / cover / scatter piece's stats** — HP, armor, and clearance band.
///
/// Shared by the three structural piece kinds that may be shot apart and that a
/// round must fly higher than to clear. Mirrors the existing [`crate::cover::CoverEntry`] shape
/// (the wall-or-prop "one shape for both" precedent). The REUSED newtypes are those
/// already in `CoverEntry`:
/// - [`CoverHp`] — structural HP (wall/cover/scatter all use the cover HP pool).
/// - [`ArmorProtection`] / [`ArmorHardness`] — same armor model as a ganger.
/// - [`HeightBand`] — LOW / MID / HIGH clearance (the §3 banding the march reads).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
pub struct StructuralSpec {
    /// The full structural HP the piece seeds to (REUSE [`CoverHp`]; starting point,
    /// tunable — not pinned by tests per the brittle-test rule).
    pub max_hp:           CoverHp,
    /// The damage-reduction stat — same armor model as a ganger (REUSE
    /// [`ArmorProtection`]).
    pub armor_protection: ArmorProtection,
    /// The penetration this piece shrugs off — same armor model as a ganger
    /// (REUSE [`ArmorHardness`]).
    pub armor_hardness:   ArmorHardness,
    /// The clearance band this piece occupies — LOW / MID / HIGH — which a
    /// round must fly strictly higher than to clear it (resolution.md §3;
    /// REUSE [`HeightBand`]).
    pub height_band:      HeightBand,
}

/// A **slab piece's stats** — HP and armor, NO height band.
///
/// A slab carries no height band because it spans the whole z-boundary; the march
/// stops on an intact slab at the boundary regardless of the shot's band
/// (`resolution.md §2`). Uses [`SlabHp`] (distinct from [`CoverHp`] — no-bare-types
/// rule 3: a slab HP pool is not a cover HP pool) and the same
/// [`ArmorProtection`] / [`ArmorHardness`] the ganger / cover models reuse.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
pub struct SlabPieceSpec {
    /// The full structural HP the slab seeds to (REUSE [`SlabHp`], the distinct
    /// slab HP pool; starting point, tunable — not pinned by tests).
    pub max_hp:           SlabHp,
    /// The damage-reduction stat — same armor model as a ganger and cover
    /// (REUSE [`ArmorProtection`]).
    pub armor_protection: ArmorProtection,
    /// The penetration this slab shrugs off — same armor model as a ganger and
    /// cover (REUSE [`ArmorHardness`]).
    pub armor_hardness:   ArmorHardness,
}
