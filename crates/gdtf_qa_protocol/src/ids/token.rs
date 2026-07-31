//! Entity **token** newtypes — the wire carriers for a live sim `Entity` (GTW-734).
//!
//! Each token is a `u64` that carries a Bevy `Entity`'s bit pattern
//! (`Entity::to_bits`). This crate does NOT depend on `bevy_ecs`, so the `u64`
//! contract is documented rather than typed against `Entity`: the game side
//! (`net_qa`, T3) mints a token with `entity.to_bits()` when it hands an entity out
//! in a [`view`](crate::view), and resolves it back with `Entity::try_from_bits` +
//! a liveness check when a [`NetIntent`](crate::intent::NetIntent) targets it (the
//! GTW-694 ruling: `from_bits` PANICS on a stale handle, so the resolve MUST be the
//! fallible `try_from_bits` and re-validate the entity still exists). A QA client
//! treats a token as an OPAQUE handle — it only ever echoes back a token a view
//! handed it, never fabricates one.

use bevy_derive::Deref;
use serde::{Deserialize, Serialize};

/// A wire handle for a **ganger** entity — a `u64` carrying its `Entity::to_bits`
/// pattern.
///
/// Handed out by every [`GangerView`](crate::view::GangerView) and echoed back by the
/// entity-targeted [`NetIntent`](crate::intent::NetIntent) variants (Select / Shove /
/// Stabilize / Execute / …). Opaque to the client; the game resolves it via
/// `Entity::try_from_bits` + liveness. A private-inner newtype (no-bare-types), serde-
/// transparent so it rides the wire as its bare `u64`.
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct GangerToken(u64);

impl GangerToken {
    /// Build a ganger token from an `Entity::to_bits` value.
    #[must_use]
    pub const fn new(bits: u64) -> Self {
        Self(bits)
    }
}

/// A wire handle for a **door** (openable) entity — a `u64` carrying its
/// `Entity::to_bits` pattern.
///
/// Handed out in the [`TerrainSummaryView`](crate::view::TerrainSummaryView) door list
/// and echoed back by [`NetIntent::OpenDoor`](crate::intent::NetIntent::OpenDoor).
/// Without this handout the open-door intent would be dead wire surface (GTW-694). A
/// private-inner newtype, serde-transparent.
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DoorToken(u64);

impl DoorToken {
    /// Build a door token from an `Entity::to_bits` value.
    #[must_use]
    pub const fn new(bits: u64) -> Self {
        Self(bits)
    }
}

/// A wire handle for a weapon **emplacement** entity — a `u64` carrying its
/// `Entity::to_bits` pattern.
///
/// Handed out in the [`TerrainSummaryView`](crate::view::TerrainSummaryView)
/// emplacement list and echoed back by the enter / exit emplacement
/// [`NetIntent`](crate::intent::NetIntent) variants. Without this handout those intents
/// would be dead wire surface (GTW-694). A private-inner newtype, serde-transparent.
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EmplacementToken(u64);

impl EmplacementToken {
    /// Build an emplacement token from an `Entity::to_bits` value.
    #[must_use]
    pub const fn new(bits: u64) -> Self {
        Self(bits)
    }
}

/// A wire handle for a **focus-target** entity — a `u64` carrying its `Entity::to_bits`
/// pattern.
///
/// Handed out in the [`PanelButtonView`](crate::view::PanelButtonView) HUD-button list
/// (GTW-789) and echoed back by
/// [`NetIntent::SetFocus`](crate::intent::NetIntent::SetFocus) to point the game's UI input
/// focus at an entity directly (GTW-783). Without the button handout `SetFocus` had no
/// panel-button token to name, so GTW-782's focus ring was un-drivable over the wire.
/// Unlike the ganger / door / emplacement tokens, a focus target is any UI entity (a HUD
/// button), so it wears its own concept name (no-bare-types rule 3). Opaque to the client;
/// the game resolves it via `Entity::try_from_bits` + a liveness check, fail-closed. A
/// private-inner newtype, serde-transparent.
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct FocusTargetNet(u64);

impl FocusTargetNet {
    /// Build a focus-target token from an `Entity::to_bits` value.
    #[must_use]
    pub const fn new(bits: u64) -> Self {
        Self(bits)
    }
}
