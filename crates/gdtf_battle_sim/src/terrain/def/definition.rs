//! The unified terrain **definition** struct — [`TerrainDef`] — and its
//! [`TerrainDisplayName`] newtype (GTW-484).

use bevy::{prelude::Deref, reflect::TypePath};
use serde::{Deserialize, Serialize};

use super::{LosBlocking, TerrainPresenterKind, TerrainSimKind, TerrainTag, TerrainUuid};

/// A terrain definition's **human-readable display name** — the label shown for a
/// terrain piece in tooling / authoring.
///
/// A name newtype over [`String`] (no-bare-types rule 1: a display name is a domain
/// value). Private inner + derived [`Deref`]; `#[serde(transparent)]` parses a bare
/// RON string. Distinct from the legacy [`TerrainName`](crate::terrain::piece::TerrainName)
/// (a registry/file key): a display name is for humans, the UUID is the key.
#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize, TypePath)]
#[serde(transparent)]
pub struct TerrainDisplayName(String);

impl TerrainDisplayName {
    /// Build a display name from its label string.
    #[must_use]
    pub const fn new(name: String) -> Self {
        Self(name)
    }
}

/// An authored **path-blocking override** value (GTW-587) — the payload of a
/// [`TerrainDef::blocks_pathing`] `Some(_)`: `true` forces the def to block pathfinding,
/// `false` forces it walkable, WINNING outright over the tag-∪-kind default.
///
/// A named newtype over `bool` (no-bare-types: an authored path-blocking decision is a domain
/// value, not a bare boolean). Private inner + derived [`Deref`] (house style);
/// `#[serde(transparent)]` round-trips it as the bare boolean wire form, so an authored
/// `blocks_pathing: Some(true)` / `Some(false)` parses and re-serializes byte-identically to the
/// pre-newtype `Option<bool>` shape (and an omitted field stays `None`, RON-invisible for every
/// shipped def). Resolved into the sim's typed
/// [`PathBlocked`](crate::occupancy::PathBlocked) answer by
/// [`derives_path_blocking`](crate::terrain::def::derives_path_blocking).
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(transparent)]
pub struct BlocksPathingOverride(bool);

impl BlocksPathingOverride {
    /// Build a path-blocking override from its boolean state — `true` forces blocking,
    /// `false` forces walkable.
    #[must_use]
    pub const fn new(over: bool) -> Self {
        Self(over)
    }
}

/// The **unified terrain definition** — the UUID-keyed terrain model (GTW-484, child T02
/// of the GTW-476 refactor) and the SOLE terrain model after GTW-496.
///
/// One definition splits cleanly into a SIM half and a PRESENTER half, plus the
/// sim-owned tags:
/// - [`key`](TerrainDef::key) — the stable [`TerrainUuid`] the registry / themes /
///   prefabs reference it by.
/// - [`display_name`](TerrainDef::display_name) — the human label ([`TerrainDisplayName`]).
/// - [`sim_kind`](TerrainDef::sim_kind) — the [`TerrainSimKind`] structural stats the
///   combat path reads (HP / armor / band).
/// - [`presenter_kind`](TerrainDef::presenter_kind) — the [`TerrainPresenterKind`]
///   presentation hooks (graphic role + optional slab footfall); the presenter reads
///   this and, by the one-way sim→presenter dependency, never the sim half or the tags.
/// - [`tags`](TerrainDef::tags) — the SIM-OWNED [`TerrainTag`]s that drive
///   `Pathing`/`FoV`/`LoS`. They sit on the sim side (a sibling field on `TerrainDef`,
///   NOT on [`TerrainPresenterKind`]) because they are presentation-agnostic sim data.
///   `#[serde(default)]` makes an omitted `tags` field parse as the EMPTY vec.
///
/// Derives [`Serialize`] / [`Deserialize`] (so a definition round-trips through RON)
/// and [`TypePath`] (so it can ride a reflected asset payload like the legacy specs).
///
/// **Not `Copy`** — [`TerrainDisplayName`], [`TerrainPresenterKind`] (its graphic
/// keys), and the `tags` [`Vec`] own heap data; it is `Clone` so the registry can hold
/// definitions by value.
///
/// Derives [`PartialEq`] but NOT [`Eq`] (GTW-547): the optional
/// [`on_death`](TerrainDef::on_death) [`OnDeathEffect`](crate::effects::on_death::OnDeathEffect) may carry
/// an [`Explode`](crate::effects::on_death::OnDeathEffect::Explode) whose
/// [`HitType`](crate::weapon::HitType) `Cone` half-angle is an `f32` (not `Eq`). A definition is
/// compared with `==` in tests, never keyed in a set.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, TypePath)]
pub struct TerrainDef {
    /// The stable UUID key the registry / themes / prefabs reference this definition by.
    pub key:            TerrainUuid,
    /// The human-readable display name (tooling / authoring label).
    pub display_name:   TerrainDisplayName,
    /// The SIM half — the structural stats the combat path reads.
    pub sim_kind:       TerrainSimKind,
    /// The PRESENTER half — the presentation hooks (graphic role + optional slab
    /// footfall); the presenter reads this and never the sim half.
    pub presenter_kind: TerrainPresenterKind,
    /// The SIM-OWNED tags that drive `Pathing`/`FoV`/`LoS` — on the sim side of the
    /// definition, NEVER on [`TerrainPresenterKind`]. Defaults to EMPTY when omitted
    /// (`#[serde(default)]`). Consumption is GTW-482, not this slice.
    #[serde(default)]
    pub tags:           Vec<TerrainTag>,
    /// The optional **on-death effect** a piece of this terrain fans when it is DESTROYED
    /// (GTW-547, child GTW-41g) — an [`OnDeathEffect`](crate::effects::on_death::OnDeathEffect)
    /// (`Explode` / `LeaveField`), e.g. a fuel barrel that leaves a burning field when smashed.
    /// `#[serde(default)]` (defaulting to `None`) so an omitted field is a piece with no death
    /// effect: the field is OPT-IN (the `tags` `#[serde(default)]` precedent), so EVERY existing
    /// terrain `.ron` — none of which author it — deserializes BYTE-IDENTICAL. Only a
    /// destructible cover-like kind (`Cover` / `Wall` / `Emplacement`) meaningfully fires it;
    /// [`setup_battle`](crate::situation::setup_battle) seeds each cover piece's effect (keyed by
    /// cell) into the [`CoverOnDeathRegistry`](crate::effects::on_death::CoverOnDeathRegistry) that
    /// [`resolve_on_death`](crate::effects::on_death::resolve_on_death) reads on a
    /// [`CoverDestroyed`](crate::occupancy_sync::CoverDestroyed).
    #[serde(default)]
    pub on_death:       Option<crate::effects::on_death::OnDeathEffect>,
    /// An OPTIONAL authored **path-blocking OVERRIDE** (GTW-587) — `Some(true)` forces this
    /// def to block pathfinding, `Some(false)` forces it walkable, and `None` (the serde
    /// default) falls back to the KIND-derived default
    /// ([`sim_kind_blocks_path`](crate::terrain::def::sim_kind_blocks_path): `Wall` / `Cover`
    /// / `Emplacement` block, `Slab` does not) UNIONED with the additive
    /// [`BlocksPathfinding`](TerrainTag::BlocksPathfinding) tag. When `Some`, the override WINS
    /// outright over both the tag and the kind default.
    ///
    /// This lets path-blocking VARY per-def independently of the kind — a low railing that bars
    /// footfall, or a decorative wall you can walk through — WITHOUT a new sim kind (the
    /// terrain-authoring "Step 3" code excursion this ticket kills). `#[serde(default)]` keeps
    /// every shipped `.ron` (none of which author it) byte-identical: an omitted field is
    /// `None` = pure kind default. Resolved by
    /// [`derives_path_blocking`](crate::terrain::def::derives_path_blocking) at battle setup.
    ///
    /// (`Option<bool>` is the authored shape the GTW-587 design pins — a tri-state
    /// force-on / force-off / use-default; the resolved value flows through the sim's typed
    /// [`BlocksPathfinding`](crate::terrain::entity::BlocksPathfinding) marker.)
    #[serde(default)]
    pub blocks_pathing: Option<BlocksPathingOverride>,
    /// An OPTIONAL authored **line-of-sight blocking OVERRIDE** (GTW-587) — a [`LosBlocking`]
    /// mode (`Full` / `UpToHeightBand` / `None`) that overrides the KIND-derived
    /// vision-occlusion default, or `None` (the serde default) to fall back to that default
    /// (`Wall` → `Full`; `Cover` / `Emplacement` → `UpToHeightBand` at the def's `height_band`;
    /// `Slab` → no occlusion) UNIONED with the additive [`BlocksVision`](TerrainTag::BlocksVision)
    /// tag. When `Some`, the override WINS outright.
    ///
    /// Line-of-sight blocking is HEIGHT-BANDED (cover occludes up to its band; a wall occludes
    /// the whole storey), so the override is the [`LosBlocking`] enum, NOT a flat bool — a glass
    /// wall (blocks pathing, not sight) authors `blocks_pathing: Some(true)` +
    /// `blocks_los: Some(None)`.
    /// `#[serde(default)]` keeps shipped `.ron` byte-identical. Resolved by
    /// [`derives_vision_occlusion`](crate::terrain::def::derives_vision_occlusion) at battle
    /// setup into the sim's height-aware
    /// [`VisionBlocking`](crate::occupancy::VisionBlocking) occluder surface.
    #[serde(default)]
    pub blocks_los:     Option<LosBlocking>,
}
