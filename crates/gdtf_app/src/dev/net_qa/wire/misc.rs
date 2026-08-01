//! The remaining wire vocabulary — fire-mode index, situation ref, seed, frame delay,
//! request id (GTW-734, moved here by GTW-943), and the procgen-stepper drive command
//! (GTW-944).
//!
//! What the other files do not own: the scalars that are not coordinates, entity tokens or
//! act-log vocabulary, plus [`StepperCommandNet`] and its [`AutoRunNet`] flag, which have no
//! other family here. Each scalar is a private-inner newtype with a derived `Deref` and a
//! `new` constructor (no-bare-types), serde-transparent so it rides the wire as its bare
//! inner.
//!
//! Each doc below says where a caller GETS a value of it — the same provenance the tokens
//! in [`token`](super::token) owe, because a `u32` index is exactly as opaque on the wire
//! as a `u64` handle (`04-critiques.md` #2 names `FireModeIndex` alongside the four
//! tokens).

use bevy::prelude::Deref;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// An index into a ganger weapon's authored fire-mode list — selects WHICH mode the C10
/// `act.fire` fires.
///
/// **Read one from** the C6 `battle.roster`, whose per-ganger card carries the weapon's
/// fire-mode list in index order. The wire never carries the resolved `FireModeSpec` (its
/// `f32` numbers are sim-side balance data): a client picks an index off that list and the
/// game side resolves it back to the real spec, fail-closed on an out-of-range index. The
/// C13 `battle.set_fire_mode` takes one too, to change the selected mode rather than
/// override one shot.
///
/// A private-inner newtype, serde-transparent over `u32` (a stable wire width for a small
/// list index).
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct FireModeIndex(u32);

impl FireModeIndex {
    /// Build a fire-mode index from its position in the weapon's mode list.
    #[must_use]
    pub const fn new(index: u32) -> Self {
        Self(index)
    }
}

/// A reference to a **situation** the game starts a battle from.
///
/// **Read one from** the C3 `situations.list`, which publishes the legal values — the whole
/// reason that read exists. Hand it back to the C15 `battle.start`. The sim loads authored
/// situations from `.ron` by name, so a ref is a name string, not a numeric id.
///
/// A name newtype over `String` (no-bare-types), serde-transparent. `Clone`-not-`Copy`.
#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct SituationRef(String);

impl SituationRef {
    /// Build a situation reference from its recipe / asset name.
    #[must_use]
    pub const fn new(name: String) -> Self {
        Self(name)
    }
}

/// A deterministic **seed** for a battle — pins the procgen RNG so a run is reproducible.
///
/// Both directions: a caller MAY choose one as the C15 `battle.start` argument, and that
/// command's reply carries the seed the run actually used (`seed_used`) — resolved as
/// override, then `GDTF_BATTLE_SEED`, then the wall clock — so a caller that omitted it can
/// still replay the battle it got.
///
/// A private-inner newtype (no-bare-types), serde-transparent over `u64`.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct SeedNet(u64);

impl SeedNet {
    /// Build a battle seed from its raw value.
    #[must_use]
    pub const fn new(seed: u64) -> Self {
        Self(seed)
    }
}

/// How many frames a frame-exact capture waits, after the command it rides has run, before
/// the game captures the shot (GTW-749).
///
/// A caller CHOOSES it; no command publishes one, and none takes one yet — the `capture`
/// rider on `run` carries only a file stem today
/// ([`CaptureRider`](gdtf_qa_protocol::command::CaptureRider)), and C2 is the row that
/// builds that rider for real. The game must count these frames itself, because
/// request/response latency cannot land on a chosen frame; counting them is what lets a
/// client catch a transient effect — a muzzle flash, an impact flash — mid-animation rather
/// than settled. `0` means "capture on the very next frame after the command ran".
///
/// A private-inner newtype (no-bare-types), serde-transparent over `u32`.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct FrameDelay(u32);

impl FrameDelay {
    /// Build a frame delay from its frame count.
    #[must_use]
    pub const fn new(frames: u32) -> Self {
        Self(frames)
    }
}

/// A client-chosen **correlation id** for a request/response pair.
///
/// A caller CHOOSES it; no command publishes one and no command takes one. It is not wired
/// into the [`message`](gdtf_qa_protocol::message) request/response variants and no C-phase
/// row wires it in, because the transport answers one call at a time — a reply cannot be
/// confused with another call's. It stays defined so a courier that ever multiplexes has a
/// typed handle to correlate with rather than a bare integer.
///
/// A private-inner newtype (no-bare-types), serde-transparent over `u64`.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct RequestId(u64);

impl RequestId {
    /// Build a request id from its raw correlation value.
    #[must_use]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }
}

/// Whether the DEV procgen stepper's **Auto** free-run is turned on.
///
/// A caller CHOOSES it; no command publishes one. It is the `running` field of
/// [`StepperCommandNet::Auto`], which sets Auto to an ABSOLUTE state rather than flipping it,
/// exactly as the stepper panel's two Start / Stop buttons do — an assignment made twice ends
/// at the same value, so a repeated call is harmless.
///
/// A private-inner newtype over `bool` (no-bare-types: `true` = free-run on, `false` = off),
/// serde-transparent.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct AutoRunNet(bool);

impl AutoRunNet {
    /// Build an Auto free-run setting — `true` to start free-running, `false` to stop.
    #[must_use]
    pub const fn new(running: bool) -> Self {
        Self(running)
    }
}

/// One DEV procgen-stepper drive command — the wire mirror of the stepper panel's three
/// egui controls.
///
/// A caller CHOOSES it; no command publishes one. It is the `command` argument of the C3
/// `procgen.step`, which is available only while a staged generation is in flight. The game
/// side maps each variant onto the SAME latch a panel button writes (`PendingStepCommand` /
/// `AutoRunning`), to be applied by the drive on its next pass — the command never advances
/// a stage itself. An independent, closed serde enum, never a leak of the game's own
/// `StepCommand`. Deleted from the shared protocol crate with the rest of the old QA API
/// (GTW-943) and re-minted here, where the command that takes it lives.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub enum StepperCommandNet {
    /// Advance exactly one stage (the panel's Next button).
    Next,
    /// Set the Auto free-run toggle to an absolute state (the panel's Start / Stop Auto
    /// buttons).
    Auto {
        /// Whether Auto free-run is turned on.
        running: AutoRunNet,
    },
    /// Drive every remaining stage to completion immediately (the panel's Skip button).
    Skip,
}
