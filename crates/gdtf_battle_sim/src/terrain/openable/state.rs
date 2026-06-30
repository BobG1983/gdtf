//! The per-openable-entity **state components** — [`OpenState`] (the closed/open toggle
//! state) and [`OpenableBlocking`] (the band a closed door occludes vision at, so the toggle
//! can re-insert [`BlocksVision`](crate::terrain::entity::BlocksVision) on close without
//! re-reading the def). GTW-503, child 482c of the tag-driven-terrain epic GTW-482.

use bevy::prelude::{Component, Deref};

use crate::cover::HeightBand;

/// The open/closed state of an **openable** terrain piece — a door / hatch (GTW-503 C1).
///
/// A NAMED CLOSED enum [`Component`] (no-bare-types: a door's open/closed state is a domain
/// value, NOT a bare `bool` — an open door reads `OpenState::Open`, never `true`). Attached at
/// terrain-entity spawn ONLY to entities whose [`TerrainDef`](crate::terrain::def::TerrainDef)
/// carries the [`Openable`](crate::terrain::def::TerrainTag::Openable) tag, and DEFAULT
/// [`Closed`](OpenState::Closed) at spawn (a door starts shut).
///
/// The toggle ([`apply_openable_toggle`](super::apply_openable_toggle)) flips this and drives
/// the GTW-501 / GTW-502 blocking components:
///
/// - [`Closed`](OpenState::Closed) — the door blocks BOTH path AND vision: the entity carries
///   [`BlocksPathfinding`](crate::terrain::entity::BlocksPathfinding) AND
///   [`BlocksVision`](crate::terrain::entity::BlocksVision) (at the band recorded in
///   [`OpenableBlocking`]), EVEN IF its `sim_kind` would not block by default (a closed slab
///   hatch still bars footfall + sight — C2).
/// - [`Open`](OpenState::Open) — the door clears BOTH: the toggle REMOVES both components, so
///   the GTW-501 / GTW-502 change-detection re-projects the surfaces and a path routes through
///   + a sightline passes.
///
/// `Default` is [`Closed`](OpenState::Closed): a freshly-spawned door is shut (C1).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum OpenState {
    /// The door is shut — it blocks path AND vision (the GTW-501 / GTW-502 components are
    /// present). The spawn default.
    #[default]
    Closed,
    /// The door is open — it clears path AND vision (the GTW-501 / GTW-502 components are
    /// removed).
    Open,
}

impl OpenState {
    /// Whether this state is [`Open`](OpenState::Open) — the door clears path + vision.
    #[must_use]
    pub const fn is_open(self) -> bool {
        matches!(self, Self::Open)
    }

    /// The OTHER state — [`Open`](OpenState::Open) ⇄ [`Closed`](OpenState::Closed). The flip a
    /// fieldless toggle applies.
    #[must_use]
    pub const fn toggled(self) -> Self {
        match self {
            Self::Closed => Self::Open,
            Self::Open => Self::Closed,
        }
    }
}

/// The vision [`HeightBand`] a **closed** openable piece occludes at — recorded on the
/// openable entity at spawn so the toggle can re-insert
/// [`BlocksVision`](crate::terrain::entity::BlocksVision) at the right band when the door is
/// closed again (GTW-503 C2 / C3), WITHOUT re-reading the
/// [`TerrainDef`](crate::terrain::def::TerrainDef) / registry from the toggle system.
///
/// A single-field tuple [`Component`] wrapping a [`HeightBand`] (no-bare-types: the band is the
/// domain value the component carries, exposed read-only through the derived [`Deref`] — the
/// inner is private, constructed via [`new`](OpenableBlocking::new)). Mirrors
/// [`BlocksVision`](crate::terrain::entity::BlocksVision)'s shape; it is the PERSISTENT band
/// record (it stays on the entity across open/close cycles, unlike `BlocksVision` which the
/// toggle removes when the door opens). The band is derived once at spawn by
/// [`closed_openable_vision_band`](crate::terrain::def::closed_openable_vision_band): the
/// `sim_kind`'s own band for a `Wall`/`Cover`, else [`HeightBand::High`] for a slab hatch.
#[derive(Component, Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OpenableBlocking(HeightBand);

impl OpenableBlocking {
    /// Build the closed-state vision-band record for an openable piece — the band derived
    /// from the def by
    /// [`closed_openable_vision_band`](crate::terrain::def::closed_openable_vision_band).
    #[must_use]
    pub const fn new(band: HeightBand) -> Self {
        Self(band)
    }
}
