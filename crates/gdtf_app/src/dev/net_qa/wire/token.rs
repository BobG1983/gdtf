//! Entity **token** newtypes — the wire carriers for a live sim `Entity` (GTW-734).
//!
//! Each token is a `u64` that carries a Bevy `Entity`'s bit pattern
//! (`Entity::to_bits`). This crate does NOT depend on `bevy_ecs`, so the `u64`
//! contract is documented rather than typed against `Entity`: the game side
//! (`net_qa`, T3) mints a token with `entity.to_bits()` when it hands an entity out
//! in a read command's reply, and resolves it back with `Entity::try_from_bits` +
//! a liveness check when a [`NetIntent`](super::act::NetIntent) targets it (the
//! GTW-694 ruling: `from_bits` PANICS on a stale handle, so the resolve MUST be the
//! fallible `try_from_bits` and re-validate the entity still exists). A QA client
//! treats a token as an OPAQUE handle — it only ever echoes back a token a read
//! handed it, never fabricates one.
//!
//! The "handed out by" sentences below are the CONTRACT each token is for, not a claim about
//! today: no read hands one out yet, because GTW-943 deleted the requests that did and the
//! commands that replace them are later tickets. See [`wire`](super)'s module doc.

use bevy::prelude::Deref;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A wire handle for a **ganger** entity — a `u64` carrying its `Entity::to_bits`
/// pattern.
///
/// Handed out by the roster read and echoed back by the
/// entity-targeted [`NetIntent`](super::act::NetIntent) variants (Select / Shove /
/// Stabilize / Execute / …). Opaque to the client; the game resolves it via
/// `Entity::try_from_bits` + liveness. A private-inner newtype (no-bare-types), serde-
/// transparent so it rides the wire as its bare `u64`.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub(crate) struct GangerToken(u64);

impl GangerToken {
    /// Build a ganger token from an `Entity::to_bits` value.
    #[must_use]
    pub(crate) const fn new(bits: u64) -> Self {
        Self(bits)
    }
}

/// A wire handle for a **door** (openable) entity — a `u64` carrying its
/// `Entity::to_bits` pattern.
///
/// Handed out in the terrain read's door list
/// and echoed back by [`NetIntent::OpenDoor`](super::act::NetIntent::OpenDoor).
/// Without this handout the open-door intent would be dead wire surface (GTW-694). A
/// private-inner newtype, serde-transparent.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub(crate) struct DoorToken(u64);

impl DoorToken {
    /// Build a door token from an `Entity::to_bits` value.
    #[must_use]
    pub(crate) const fn new(bits: u64) -> Self {
        Self(bits)
    }
}

/// A wire handle for a weapon **emplacement** entity — a `u64` carrying its
/// `Entity::to_bits` pattern.
///
/// Handed out in the terrain read's
/// emplacement list and echoed back by the enter / exit emplacement
/// [`NetIntent`](super::act::NetIntent) variants. Without this handout those intents
/// would be dead wire surface (GTW-694). A private-inner newtype, serde-transparent.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub(crate) struct EmplacementToken(u64);

impl EmplacementToken {
    /// Build an emplacement token from an `Entity::to_bits` value.
    #[must_use]
    pub(crate) const fn new(bits: u64) -> Self {
        Self(bits)
    }
}

/// A wire handle for a **focus-target** entity — a `u64` carrying its `Entity::to_bits`
/// pattern.
///
/// Handed out in the HUD-button read's list
/// (GTW-789) and echoed back by
/// [`NetIntent::SetFocus`](super::act::NetIntent::SetFocus) to point the game's UI input
/// focus at an entity directly (GTW-783). Without the button handout `SetFocus` had no
/// panel-button token to name, so GTW-782's focus ring was un-drivable over the wire.
/// Unlike the ganger / door / emplacement tokens, a focus target is any UI entity (a HUD
/// button), so it wears its own concept name (no-bare-types rule 3). Opaque to the client;
/// the game resolves it via `Entity::try_from_bits` + a liveness check, fail-closed. A
/// private-inner newtype, serde-transparent.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub(crate) struct FocusTargetNet(u64);

impl FocusTargetNet {
    /// Build a focus-target token from an `Entity::to_bits` value.
    #[must_use]
    pub(crate) const fn new(bits: u64) -> Self {
        Self(bits)
    }
}
