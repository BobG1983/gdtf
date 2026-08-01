//! The remaining scalar handles — fire-mode index, event cap, situation ref, seed, frame
//! delay, request id (GTW-734, moved here by GTW-943).
//!
//! The wire scalars that are not coordinates or entity tokens. Each is a private-inner
//! newtype with a derived `Deref` and a `new` constructor (no-bare-types), serde-
//! transparent so it rides the wire as its bare inner.
//!
//! Each doc below names the call the scalar is FOR. None of those calls exists yet: GTW-943
//! deleted the requests that took them and the commands that replace them are later tickets.
//! See [`wire`](super)'s module doc.

use bevy::prelude::Deref;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// An index into a ganger weapon's authored fire-mode list — selects WHICH mode a
/// [`NetIntent::Fire`](super::act::NetIntent::Fire) fires.
///
/// The wire never carries the resolved `FireModeSpec` (its `f32` numbers are sim-side
/// balance data): a QA client reads the indexed mode list off the weapon read, picks an
/// index, and the game side resolves
/// it back to the real spec. A private-inner newtype, serde-transparent over `u32` (a
/// stable wire width for a small list index).
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub(crate) struct FireModeIndex(u32);

impl FireModeIndex {
    /// Build a fire-mode index from its position in the weapon's mode list.
    #[must_use]
    pub(crate) const fn new(index: u32) -> Self {
        Self(index)
    }
}

/// A cap on how many events an event drain
/// takes — the client's back-pressure knob.
///
/// A private-inner newtype (no-bare-types), serde-transparent over `u32`. When a
/// request omits it (`None`) the game drains the whole outbox.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub(crate) struct EventCap(u32);

impl EventCap {
    /// Build an event cap from its maximum count.
    #[must_use]
    pub(crate) const fn new(max: u32) -> Self {
        Self(max)
    }
}

/// A reference to a **situation** the game should start a battle from — a start-battle
/// call names it (GTW-734, the T9
/// navigation surface defined now so T9 does not churn the wire).
///
/// An opaque situation handle (a recipe / asset name the game resolves): the sim loads
/// authored situations from `.ron` by name, so a ref is a name string, not a numeric
/// id. A name newtype over `String` (no-bare-types), serde-transparent.
/// `Clone`-not-`Copy`.
#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub(crate) struct SituationRef(String);

impl SituationRef {
    /// Build a situation reference from its recipe / asset name.
    #[must_use]
    pub(crate) const fn new(name: String) -> Self {
        Self(name)
    }
}

/// An optional deterministic **seed** for a start-battle call — pins the procgen RNG so a
/// QA run is reproducible (GTW-734, the T9 navigation surface).
///
/// A private-inner newtype (no-bare-types), serde-transparent over `u64`. When a
/// request omits it (`None`) the game picks its own seed.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub(crate) struct SeedNet(u64);

impl SeedNet {
    /// Build a battle seed from its raw value.
    #[must_use]
    pub(crate) const fn new(seed: u64) -> Self {
        Self(seed)
    }
}

/// The number of frames a frame-exact capture waits, after its
/// embedded intent is queued, before the game captures the deferred shot (GTW-749).
///
/// The game counts these frames itself (request/response latency cannot land on a
/// specific frame), so a client can catch a transient effect — a muzzle flash, an
/// impact flash — mid-animation rather than settled. `0` means "capture on the very
/// next frame after the intent is queued". A private-inner newtype (no-bare-types),
/// serde-transparent over `u32`.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub(crate) struct FrameDelay(u32);

impl FrameDelay {
    /// Build a frame delay from its frame count.
    #[must_use]
    pub(crate) const fn new(frames: u32) -> Self {
        Self(frames)
    }
}

/// A client-chosen **correlation id** for a request/response pair — lets the T8 MCP
/// bridge match a reply to the call that produced it (GTW-734, the "`RequestId` if
/// needed" id).
///
/// Not wired into the [`message`](gdtf_qa_protocol::message) request/response variants (the T3
/// transport is one-at-a-time request/response); defined here so the T8 bridge has a
/// typed correlation handle when it multiplexes. A private-inner newtype
/// (no-bare-types), serde-transparent over `u64`.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub(crate) struct RequestId(u64);

impl RequestId {
    /// Build a request id from its raw correlation value.
    #[must_use]
    pub(crate) const fn new(id: u64) -> Self {
        Self(id)
    }
}
