//! The unified terrain **definition** struct — [`TerrainDef`] — and its
//! [`TerrainDisplayName`] newtype (GTW-484).

use bevy::{prelude::Deref, reflect::TypePath};
use serde::{Deserialize, Serialize};

use super::{TerrainPresenterKind, TerrainSimKind, TerrainTag, TerrainUuid};

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
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, TypePath)]
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
}
