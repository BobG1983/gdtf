//! The armor-wear implementation — the [`ArmorBroken`] message + the pure [`wear_armor`]
//! crossing-detection helper. See the module docs (`super`) for the emit-once rule.

use bevy::prelude::{Entity, Message};

use crate::{
    armor::{ArmorIntegrity, BodyPart},
    resolve_hit::IntegrityWear,
};

/// A worn armor piece **was damaged** — its [`crate::armor::ArmorIntegrity`] was
/// reduced by `delta` at the struck [`BodyPart`] on a given ganger, **without**
/// breaking (it stayed protecting, `integrity > 0`) — the GTW-313 companion to
/// [`ArmorBroken`].
///
/// Emitted on **each** wearing hit that reduces a still-protecting piece short of
/// breaking it (`delta > 0` and the piece still protects after) — the
/// integrity-loss-per-hit signal the presenter draws an `"Armor -N"` pop from. It
/// is **mutually exclusive** with [`ArmorBroken`] per hit ([`wear_armor`]'s
/// [`ArmorWearOutcome`]): the breaking hit emits [`ArmorBroken`] (the crossing),
/// every prior wearing hit emits [`ArmorDamaged`]; a hit on an already-broken /
/// bare-flesh piece, or a zero-wear hit, emits NEITHER. So this fires `0..n`
/// times before the single [`ArmorBroken`], never on the same hit as it.
///
/// A buffered Bevy **message** (`#[derive(Message)]`), mirroring [`ArmorBroken`] /
/// [`crate::occupancy_sync::CoverDestroyed`] — NOT the observer `Event` API
/// (`bevy-traps.md` #4). The payload is named: [`BodyPart`] is a domain
/// newtype-enum, [`delta`](ArmorDamaged::delta) is the [`IntegrityWear`] domain
/// newtype (no-bare-types), and [`Entity`] is the one framework-plumbing handle the
/// no-bare-types rule permits. [`Hash`] is derivable because [`IntegrityWear`]
/// (its only non-`Copy`-only field beyond [`Entity`]/[`BodyPart`]) derives `Hash`,
/// matching [`ArmorBroken`]'s derive set.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ArmorDamaged {
    /// The ganger whose armor was damaged — the entity owning the worn piece.
    pub ganger: Entity,
    /// The body location whose worn piece was reduced (still protecting after).
    pub part:   BodyPart,
    /// The [`IntegrityWear`] removed from the piece this hit (the per-hit delta).
    pub delta:  IntegrityWear,
}

impl ArmorDamaged {
    /// Build an armor-damaged signal for the `ganger` whose worn piece at `part` was
    /// reduced by `delta` this hit, short of breaking.
    #[must_use]
    pub const fn new(ganger: Entity, part: BodyPart, delta: IntegrityWear) -> Self {
        Self {
            ganger,
            part,
            delta,
        }
    }
}

/// The mutually-exclusive per-hit classification of an armor-wear application —
/// the enriched return of [`wear_armor`] (GTW-313).
///
/// A single hit's wear on a worn piece is exactly one of three outcomes, so the
/// caller maps it to **at most one** of the two armor signals (never both):
///
/// - [`Broke`](ArmorWearOutcome::Broke) — the piece was protecting and this wear
///   crossed it to broken (`integrity ≤ 0`): the single protecting→broken
///   crossing, carrying [`ArmorBroken`] (UNCHANGED from the pre-GTW-313 `Some`).
/// - [`Damaged`](ArmorWearOutcome::Damaged) — the piece was protecting, this wear
///   reduced it (`delta > 0`), and it still protects after: carries [`ArmorDamaged`].
/// - [`Unaffected`](ArmorWearOutcome::Unaffected) — neither signal fires: the
///   piece was already broken / bare flesh (not protecting before), OR the wear
///   was `0` (a no-op reduction). Emit nothing — mirroring the [`ArmorBroken`]
///   "emit only on the event" rule (an already-broken piece never re-emits, a
///   zero-wear hit never pops a misleading `"Armor -0"`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ArmorWearOutcome {
    /// No armor signal — already-broken/bare piece, or a zero-wear hit.
    Unaffected,
    /// The piece was damaged (reduced, still protecting) — the [`ArmorDamaged`] signal.
    Damaged(ArmorDamaged),
    /// The piece broke (the protecting→broken crossing) — the [`ArmorBroken`] signal.
    Broke(ArmorBroken),
}

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

/// Persist a hit's integrity `wear` onto the struck worn-armor-piece **entity's**
/// [`ArmorIntegrity`] component at `part`, returning the per-hit [`ArmorWearOutcome`]
/// — the [`ArmorBroken`] crossing, the [`ArmorDamaged`] reduction (GTW-313), or nothing.
///
/// The wear-side primitive E3.6's `apply_hit` calls (`docs/combat/
/// weapons-and-armor.md` §"Per-hit resolution" step 3). Since GTW-323 (ADR-0004) it
/// mutates the struck **piece entity's** integrity component directly (resolved by the
/// caller from `ganger → Wears → the BodyPart-tagged piece`), NOT a slot of a
/// ganger-side `WornArmor` array — the model is the same, the storage is the piece
/// entity. It:
///
/// 1. reads the **pre**-wear integrity (protecting iff `> 0`);
/// 2. subtracts `wear` from `integrity` in place (integrity may fall to `≤ 0` —
///    "useless at `≤ 0`"); and
/// 3. reads the **post**-wear integrity (broken iff `≤ 0`).
///
/// The mutation in step 2 is byte-identical to the pre-GTW-323 array-slot wear — only
/// the storage moved from a `WornArmor` slot to the piece entity's component. The
/// return classification is unchanged from GTW-313:
///
/// - protecting before AND broken after ⇒ [`ArmorWearOutcome::Broke`] — the single
///   protecting→broken crossing (the UNCHANGED break case);
/// - protecting before, NOT broken after, AND `wear > 0` ⇒
///   [`ArmorWearOutcome::Damaged`] carrying the `delta = wear` removed this hit (the
///   GTW-313 reduction signal);
/// - otherwise (NOT protecting before — already broken / bare flesh — OR `wear == 0`)
///   ⇒ [`ArmorWearOutcome::Unaffected`]: emit nothing (an already-broken piece
///   re-wearing fired its signal once already; a zero-wear hit is a no-op — neither
///   pops a misleading `"Armor -0"`). This mirrors [`ArmorBroken`]'s emit-only-on-
///   the-event rule.
///
/// The caller writes the [`ArmorBroken`] / [`ArmorDamaged`] payload to its matching
/// [`bevy::prelude::MessageWriter`] at the system boundary, keeping the "emit once /
/// emit per reduction" rule in the pure function and the buffer write outside it.
///
/// Battle-local: this only ever mutates the passed battle-local piece-entity
/// [`ArmorIntegrity`], never the roster [`crate::armor::SourceArmor`] /
/// [`crate::armor::ArmorSpec`] it was seeded from (the model/view separation; ADR-0001,
/// `docs/decisions/0001-rust-bevy-rewrite.md`).
#[must_use]
pub fn wear_armor(
    integrity: &mut ArmorIntegrity,
    part: BodyPart,
    wear: IntegrityWear,
    ganger: Entity,
) -> ArmorWearOutcome {
    // Was the piece protecting before this hit? (integrity > 0)
    let was_protecting = **integrity > 0;

    // Apply the wear in place on the battle-local piece component (may drop to ≤ 0).
    // BYTE-IDENTICAL to the pre-GTW-323 array-slot mutation — only the storage moved.
    *integrity = ArmorIntegrity::new(**integrity - *wear);

    // Is the piece broken now? (integrity ≤ 0)
    let now_broken = **integrity <= 0;

    // Classify the per-hit outcome — at most one of the two armor signals. An
    // already-broken piece (was_protecting == false) emits NEITHER; a zero-wear hit
    // on a still-protecting piece emits NEITHER (no-op reduction).
    if was_protecting && now_broken {
        // The single protecting→broken crossing (UNCHANGED behavior).
        ArmorWearOutcome::Broke(ArmorBroken::new(ganger, part))
    } else if was_protecting && *wear > 0 {
        // A reduction that did NOT break the piece — the GTW-313 wear signal.
        ArmorWearOutcome::Damaged(ArmorDamaged::new(ganger, part, wear))
    } else {
        // Already broken / bare flesh, or a zero-wear no-op: emit nothing.
        ArmorWearOutcome::Unaffected
    }
}
