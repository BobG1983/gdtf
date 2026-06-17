//! The armor-wear implementation — the [`ArmorBroken`] message + the pure [`wear_armor`]
//! crossing-detection helper. See the module docs (`super`) for the emit-once rule.

use bevy::prelude::{Entity, Message};

use crate::{
    armor::{ArmorIntegrity, BodyPart, WornArmor},
    resolve_hit::IntegrityWear,
};

/// A worn armor piece **broke** — its [`crate::armor::ArmorIntegrity`] crossed
/// from protecting (`> 0`) to useless (`≤ 0`) — at the struck [`BodyPart`] on a
/// given ganger.
///
/// Emitted **exactly once**, on the single protecting→broken crossing
/// ([`wear_armor`]); a piece already at `≤ 0` that re-wears does not re-emit.
/// The presenter (and any reactive sim system) reads this to react to the
/// armor-broken moment (`docs/combat/resolution.md` §3 names the analogous
/// cover-destroyed signal — this is its armor-piece sibling).
///
/// A buffered Bevy **message** (`#[derive(Message)]`), mirroring
/// [`crate::occupancy_sync::CoverDestroyed`] — NOT the observer `Event` API
/// (`bevy-traps.md` #4: Bevy 0.18 renamed buffered `Event`/`EventReader` to
/// `Message`/`MessageReader`), so it is written with
/// [`bevy::prelude::MessageWriter`] and read with
/// [`bevy::prelude::MessageReader`]. The payload is named — [`BodyPart`] is a
/// domain newtype-enum (no-bare-types); [`Entity`] is Bevy framework plumbing
/// (the only bare type the no-bare-types rule permits — an entity handle, not a
/// domain value).
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ArmorBroken {
    /// The ganger whose armor broke — the entity owning the worn piece.
    pub ganger: Entity,
    /// The body location whose worn piece crossed to `integrity ≤ 0`.
    pub part:   BodyPart,
}

impl ArmorBroken {
    /// Build an armor-broken signal for the `ganger` whose worn piece at `part`
    /// just crossed to broken.
    #[must_use]
    pub const fn new(ganger: Entity, part: BodyPart) -> Self {
        Self { ganger, part }
    }
}

/// Persist a hit's integrity `wear` onto the battle-local [`WornArmor`] at `part`,
/// returning the [`ArmorBroken`] signal **iff** this wear is the single
/// protecting→broken crossing.
///
/// The wear-side primitive E3.6's `apply_hit` calls (`docs/combat/
/// weapons-and-armor.md` §"Per-hit resolution" step 3). It:
///
/// 1. reads the **pre**-wear integrity at `part` (protecting iff `> 0`);
/// 2. subtracts `wear` via [`WornArmor::wear_integrity`] (integrity may fall to
///    `≤ 0` — "useless at `≤ 0`"); and
/// 3. reads the **post**-wear integrity (broken iff `≤ 0`).
///
/// It returns `Some(`[`ArmorBroken`]`)` **only** when the piece was protecting
/// before (`pre > 0`) AND is broken after (`post ≤ 0`) — the one crossing — and
/// `None` otherwise (so an already-broken piece that re-wears emits nothing: the
/// signal fired once, on the original crossing). The caller writes the `Some` to
/// a [`bevy::prelude::MessageWriter<ArmorBroken>`]; this keeps the "emit once"
/// rule in the pure function and the message-buffer write at the system boundary.
///
/// Battle-local: this only ever mutates the passed [`WornArmor`] copy, never the
/// roster [`crate::armor::SourceArmor`] it was seeded from (the model/view
/// separation; ADR-0001, `docs/decisions/0001-rust-bevy-rewrite.md`).
#[must_use]
pub fn wear_armor(
    worn: &mut WornArmor,
    part: BodyPart,
    wear: IntegrityWear,
    ganger: Entity,
) -> Option<ArmorBroken> {
    // Was the piece protecting before this hit? (integrity > 0)
    let was_protecting = worn.protects(part);

    // Apply the wear in place on the battle-local copy (may drop to ≤ 0).
    worn.wear_integrity(part, ArmorIntegrity::new(*wear));

    // Is the piece broken now? (integrity ≤ 0)
    let now_broken = !worn.protects(part);

    // Emit the signal iff this is the single protecting→broken crossing. An
    // already-broken piece (was_protecting == false) never re-emits.
    if was_protecting && now_broken {
        Some(ArmorBroken::new(ganger, part))
    } else {
        None
    }
}
