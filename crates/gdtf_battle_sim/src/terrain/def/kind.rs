//! The **kind enums + tag** of the unified terrain model (GTW-484): the SIM-side
//! [`TerrainSimKind`] (HP/armor/band structural stats), the PRESENTER-side
//! [`TerrainPresenterKind`] (graphic role + optional slab footfall, presentation
//! only), and the SIM-OWNED [`TerrainTag`].

use bevy::reflect::TypePath;
use serde::{Deserialize, Serialize};

use crate::{
    armor::{ArmorHardness, ArmorProtection},
    cover::{CoverHp, HeightBand},
    slab::SlabHp,
    terrain::piece::{FootfallSound, TerrainGraphicKey},
};

/// The **SIM half** of a terrain definition — which structural kind a piece is and
/// the combat stats the sim reads from it (HP / armor / clearance band).
///
/// Three variants only — `Wall` / `Cover` / `Slab` — the structural kinds the combat
/// path resolves. Per the GTW-476 redesign:
/// - **Scatter folds into `Cover`** (there is no `Scatter` variant; loose debris is a
///   low cover prop, same structural model).
/// - The legacy **`Floor` kind is RETIRED** (there is no `Floor` variant): a floor's
///   move cost will live on the theme `default_floor` seam, NOT on a terrain kind.
///
/// Every variant is a **STRUCT variant** so RON serialises as the named-struct form
/// (`Slab(hp: 120, ...)`), never the double-paren tuple form `Slab((...))` the legacy
/// [`TerrainKindSpec`](crate::terrain::piece::TerrainKindSpec) produced.
///
/// Each field REUSES an existing sim newtype (no-bare-types; no parallels invented):
/// [`CoverHp`] / [`SlabHp`] for the HP pools, [`ArmorProtection`] / [`ArmorHardness`]
/// for the armor model (terrain reuses the ganger armor model), and [`HeightBand`]
/// for the clearance band a round must fly higher than to clear. A slab carries **no**
/// height band — it spans the whole z-boundary (`docs/combat/resolution.md` §2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize, TypePath)]
pub enum TerrainSimKind {
    /// Solid blocking geometry — a destructible wall that fills the cell; nothing
    /// flies over it within a storey. Carries structural HP, armor, and a clearance
    /// band.
    Wall {
        /// The full structural HP the wall seeds to (REUSE [`CoverHp`]).
        hp:               CoverHp,
        /// The wall's damage-reduction stat (REUSE [`ArmorProtection`]).
        armor_protection: ArmorProtection,
        /// The penetration the wall shrugs off (REUSE [`ArmorHardness`]).
        armor_hardness:   ArmorHardness,
        /// The clearance band the wall occupies (REUSE [`HeightBand`]).
        height_band:      HeightBand,
    },
    /// Chest-high cover / loose scatter — a destructible prop a round may clear by
    /// flying higher than its band (`resolution.md` §3). Scatter folds in here: it is
    /// a low cover prop with the same structural model.
    Cover {
        /// The full structural HP the cover seeds to (REUSE [`CoverHp`]).
        hp:               CoverHp,
        /// The cover's damage-reduction stat (REUSE [`ArmorProtection`]).
        armor_protection: ArmorProtection,
        /// The penetration the cover shrugs off (REUSE [`ArmorHardness`]).
        armor_hardness:   ArmorHardness,
        /// The clearance band the cover occupies (REUSE [`HeightBand`]).
        height_band:      HeightBand,
    },
    /// Floor / roof slab — a destructible piece spanning a z-boundary. A slab carries
    /// **no** height band: the march stops on an intact slab regardless of band
    /// (`resolution.md` §2). Uses the distinct [`SlabHp`] pool.
    Slab {
        /// The full structural HP the slab seeds to (REUSE [`SlabHp`], the distinct
        /// slab HP pool).
        hp:               SlabHp,
        /// The slab's damage-reduction stat (REUSE [`ArmorProtection`]).
        armor_protection: ArmorProtection,
        /// The penetration the slab shrugs off (REUSE [`ArmorHardness`]).
        armor_hardness:   ArmorHardness,
    },
}

/// The **PRESENTER half** of a terrain definition — strictly the presentation hooks
/// the presenter resolves, mirroring the sim kinds.
///
/// PRESENTATION ONLY (no-bare-types is upheld via the reused [`TerrainGraphicKey`] /
/// [`FootfallSound`] newtypes). The one-way sim→presenter dependency means the
/// presenter reads this half and never the sim half — and this half deliberately
/// carries **NO tags** (tags are SIM-owned; see [`TerrainTag`]).
///
/// Every variant carries `graphic_name: TerrainGraphicKey` (the graphic role key the
/// presenter resolves to a tile atlas entry, REUSED — no new bare-`String` graphic
/// field), for ALL kinds including `Wall`. Only `Slab` additionally carries an
/// OPTIONAL `footfall` (`Wall` / `Cover` have no footfall field).
///
/// STRUCT variants, for the same named-struct RON shape as [`TerrainSimKind`].
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize, TypePath)]
pub enum TerrainPresenterKind {
    /// Presentation for a wall — only the graphic role key (no footfall).
    Wall {
        /// The graphic role key the presenter resolves to a tile atlas entry
        /// (REUSE [`TerrainGraphicKey`]).
        graphic_name: TerrainGraphicKey,
    },
    /// Presentation for cover / scatter — only the graphic role key (no footfall).
    Cover {
        /// The graphic role key the presenter resolves to a tile atlas entry
        /// (REUSE [`TerrainGraphicKey`]).
        graphic_name: TerrainGraphicKey,
    },
    /// Presentation for a slab — the graphic role key plus an OPTIONAL footfall sound
    /// (a slab is stepped on, so it alone has a footfall).
    Slab {
        /// The graphic role key the presenter resolves to a tile atlas entry
        /// (REUSE [`TerrainGraphicKey`]).
        graphic_name: TerrainGraphicKey,
        /// The OPTIONAL footfall sound the presenter plays when a ganger steps on the
        /// slab (REUSE [`FootfallSound`]); `None` when the slab has no authored
        /// footfall.
        footfall:     Option<FootfallSound>,
    },
}

/// A **sim-owned terrain tag** — a closed set of structural traits that drive the
/// sim's `Pathing` / `FoV` / `LoS` (GTW-476 redesign / *terrain-data-model-redesign-2026-06-28*).
///
/// Tags are SIM data because they are presentation-agnostic and feed the sim's
/// movement / vision / line-of-sight resolution; the presenter structurally cannot
/// (and must not) read them. They live on the SIM side of
/// [`TerrainDef`](super::TerrainDef) (its `tags` field), never on
/// [`TerrainPresenterKind`].
///
/// A NAMED CLOSED enum (no-bare-types: a tag is a domain value, not a bare string or
/// flag). This slice only makes the tag set EXIST and round-trip; the actual
/// `Pathing`/`FoV`/`LoS` effects + change-detection are GTW-482, NOT here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize, TypePath)]
pub enum TerrainTag {
    /// The piece can be opened (a door / hatch) — the sim treats it as passable when
    /// open and blocking when closed (consumption is GTW-482).
    Openable,
    /// The piece blocks line-of-sight / field-of-view — the sim stops vision at it
    /// (consumption is GTW-482).
    BlocksVision,
    /// The piece blocks pathfinding — the sim treats the cell as impassable
    /// (consumption is GTW-482).
    BlocksPathfinding,
    /// The piece is indestructible — combat HP depletion can never destroy it
    /// (consumption is GTW-482).
    Indestructible,
}
