//! Entity **token** newtypes — the wire carriers for a live sim `Entity` (GTW-734).
//!
//! Each token is a `u64` that carries a Bevy `Entity`'s bit pattern (`Entity::to_bits`).
//! The game side mints one with `entity.to_bits()` when it hands an entity out in a read's
//! reply, and resolves it back with `Entity::try_from_bits` + a liveness check when a
//! command targets it (the GTW-694 ruling: `from_bits` PANICS on a stale handle, so the
//! resolve MUST be the fallible `try_from_bits` and re-validate the entity still exists). A
//! QA client treats a token as an OPAQUE handle — it only ever echoes back a token a read
//! handed it, never fabricates one.
//!
//! # Where a caller gets one
//!
//! Which is the question each doc comment below answers. A derived schema over a
//! `#[serde(transparent)]` `u64` says `{"type":"integer","format":"uint64"}` and nothing
//! more, so a caller reading the catalogue alone cannot tell a ganger handle from a door
//! handle from a number it may invent — the "five opaque `u64`/index types, zero published
//! provenance" finding (`04-critiques.md` #2). Every token here therefore names the command
//! that publishes it, or says plainly that none does and which C-phase row would.

use bevy::prelude::Deref;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A wire handle for a **ganger** entity — a `u64` carrying its `Entity::to_bits` pattern.
///
/// **Read one from** the C6 `battle.roster` (its per-ganger card), the C7 `battle.visible`
/// (the gangers inside the lit area), or the C10 `act.select` reply, which answers
/// `Selected { token }`. Echo it back to the entity-targeted acts: `act.select`,
/// `act.shove`, `act.stabilize`, `act.execute`, and `act.melee`'s ganger target.
///
/// Opaque to the client; the game resolves it via `Entity::try_from_bits` + liveness,
/// fail-closed. A private-inner newtype (no-bare-types), serde-transparent so it rides the
/// wire as its bare `u64`.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct GangerToken(u64);

impl GangerToken {
    /// Build a ganger token from an `Entity::to_bits` value.
    #[must_use]
    pub const fn new(bits: u64) -> Self {
        Self(bits)
    }
}

/// A wire handle for a **door** (openable) entity — a `u64` carrying its `Entity::to_bits`
/// pattern.
///
/// **No command publishes one today.** The C12 `act.open_door` is CELL-addressed
/// (`{ at: CellLevelNet }`, refusing `NoDoorThere` when that cell holds no door), and the
/// per-cell `battle.terrain` dump that used to hand doors out was cut from the plan. The
/// read that would carry one, if a door ever needs naming by identity rather than by
/// position, is the C7 `battle.visible` — its reply already lists the doors in the lit area.
///
/// It stays in the vocabulary because a door IS an entity and a token is how this wire
/// names an entity; deleting it and re-minting it later is the churn this ticket exists to
/// avoid. A private-inner newtype, serde-transparent.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
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
/// **No command publishes one today**, for the same reason as [`DoorToken`]: the C12
/// `act.enter_emplacement` / `act.exit_emplacement` are cell-addressed, and the C7
/// `battle.visible` lists emplacements by `at` + state rather than by handle. That list is
/// where a token would appear if identity ever mattered more than position.
///
/// A private-inner newtype, serde-transparent.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
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
/// **Read one from** the C9 `ui.focus`, which reports what holds UI input focus and what
/// else can take it. Echo it back to the C14 `input.set_focus` (and `input.activate`, whose
/// target is an optional one) to point the game's focus ring at a button directly — without
/// that handout, GTW-782's focus ring was un-drivable over the wire.
///
/// Unlike the ganger / door / emplacement tokens, a focus target is any UI entity, so it
/// wears its own concept name (no-bare-types rule 3). Opaque to the client; the game
/// resolves it via `Entity::try_from_bits` + a liveness check, fail-closed. A private-inner
/// newtype, serde-transparent.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct FocusTargetNet(u64);

impl FocusTargetNet {
    /// Build a focus-target token from an `Entity::to_bits` value.
    #[must_use]
    pub const fn new(bits: u64) -> Self {
        Self(bits)
    }
}
