//! The act-log wire vocabulary — [`ActProvenanceNet`], [`LogReadCap`] and
//! [`LogDroppedCount`] (GTW-944).
//!
//! The sim's `ActLog` is the machine-readable record of what happened in a battle: an
//! ordered ring of entries, each carrying a sequence number
//! ([`ActSeqNet`](super::act::ActSeqNet), in [`act`](super::act)), why it happened (a
//! provenance), and what happened (a deed). The C8
//! `log.read` command is the read over it — `{ since, max }` in, entries plus `head`,
//! `oldest_seq` and `dropped` back.
//!
//! # What is here and what is not
//!
//! This file holds the log's SCALAR and CAUSE vocabulary. The 26-variant deed mirror is NOT
//! here, because the plan puts it in the `log.read` REPLY rather than in this shared
//! vocabulary: the C8 row reads "`log.read` (directory split) — all 26 `ActDeed` variants
//! with `seq` and `provenance`", and a directory of reply files beside that command is where
//! it lands. The sim's `ActDeed` carries a boxed shot report, an injury ledger and three
//! posture / vitals / magazine snapshots, so a mirror of it is that command's whole reply,
//! not one type other commands embed. (Were it a `wire/` type, its size would make it the
//! directory module `wire/log/` — the line bands force a SPLIT, never an omission.)
//!
//! [`LogReadCap`] is the type `develop` carried as `EventCap` in [`misc`](super::misc). It
//! is renamed, not deleted: `EventCap` capped a drain of the event outbox, and GTW-943
//! removed the outbox, while `04-critiques.md` #A4 records `EventCap` as the real type
//! behind the spec's `log.read` `EntryCap`. Its round-trip and wire-text cases moved with it
//! into `test/log.rs`.

use bevy::prelude::Deref;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::token::GangerToken;

/// Why a logged act happened — the wire mirror of the sim `ActProvenance`.
///
/// The discriminator that separates a player's own act from an enemy brain's, an
/// out-of-turn reaction interrupt, and a clock beat (a turn boundary, a bleed / DOT /
/// field tick). It is what lets a client reading a seq range tell "the shot I asked for"
/// from "the two interrupts my move drew".
///
/// A caller reads one off every C8 `log.read` entry; nothing else publishes it. An
/// independent serde enum — never a leak of the sim type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub enum ActProvenanceNet {
    /// A player-commanded act — the actor belongs to the player faction.
    Commanded,
    /// An enemy-brain act — the actor acted on its own faction's turn.
    AiTurn,
    /// A reaction-fire interrupt: the actor fired out of turn because the ganger named in
    /// `interrupted` acted in its line of sight.
    Reaction {
        /// The ganger whose act triggered the interrupt — the mover / shooter that was
        /// interrupted, by its wire token.
        interrupted: GangerToken,
    },
    /// A clock-driven fact with no commanding actor — a turn boundary, or a per-round
    /// bleed / DOT / field tick.
    Clock,
}

/// A cap on how many act-log entries one read returns — the client's back-pressure knob.
///
/// A caller CHOOSES this value; no command publishes it. It is the `max` argument of the
/// C8 `log.read`, which is a non-destructive cursor read: capping it short leaves the rest
/// in the ring for the next call rather than dropping it. Omitting it (`None`) lets the
/// host return everything from the cursor to the head.
///
/// A private-inner newtype (no-bare-types), serde-transparent over `u32`.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct LogReadCap(u32);

impl LogReadCap {
    /// Build a read cap from its maximum entry count.
    #[must_use]
    pub const fn new(max: u32) -> Self {
        Self(max)
    }
}

/// How many act-log entries the ring has dropped to overflow since the battle began — the
/// wire mirror of the sim `ActLogDropped`.
///
/// A caller reads it off the C8 `log.read` reply. It is the log-wide total, for
/// diagnostics: a client that wants to know whether IT personally fell off the window
/// compares its own cursor against the reply's `oldest_seq`
/// ([`ActSeqNet`](super::act::ActSeqNet)) instead.
///
/// A private-inner newtype (no-bare-types), serde-transparent over `u32`, matching the
/// sim's own counter width.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct LogDroppedCount(u32);

impl LogDroppedCount {
    /// Build a drop count from its total.
    #[must_use]
    pub const fn new(dropped: u32) -> Self {
        Self(dropped)
    }
}
