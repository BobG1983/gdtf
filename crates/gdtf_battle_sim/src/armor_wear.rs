//! Armor wear persistence + the armor-broken signal — the E3.5 slice (GTW-187).
//!
//! This is the **wear-side primitive** of per-hit application (`docs/combat/
//! weapons-and-armor.md` §"Per-hit resolution" step 3): a hit's computed
//! [`IntegrityWear`] (the E3.3 [`crate::resolve_hit::HitResult::wear`]) is
//! subtracted from the struck location's [`crate::armor::ArmorIntegrity`] on the
//! **battle-local** [`WornArmor`] copy, so a piece worn to `integrity ≤ 0` stops
//! protecting for the rest of the battle (later hits on that location resolve as
//! bare flesh — the [`WornArmor::protects`] gate). The worn copy is battle-local
//! (E1.3): wearing it **never** touches the roster [`crate::armor::SourceArmor`]
//! (the model/view separation; ADR-0001,
//! `docs/decisions/0001-rust-bevy-rewrite.md`).
//!
//! When a piece crosses from protecting (`integrity > 0`) to broken
//! (`integrity ≤ 0`) — **exactly once**, on that single crossing — an
//! [`ArmorBroken`] signal is emitted carrying the struck [`BodyPart`] and the
//! owning ganger [`Entity`]. A piece already at `≤ 0` that re-wears does NOT
//! re-emit (the depletion fired once already). This mirrors the
//! [`crate::occupancy_sync::CoverDestroyed`] precedent
//! (`docs/combat/resolution.md` §3's cover-destroyed signal): a buffered Bevy
//! **message** (`#[derive(Message)]` — Bevy 0.18 renamed buffered
//! `Event`/`EventReader` to `Message`/`MessageReader`, `bevy-traps.md` #4), NOT
//! the targeted/observer `Event` API.
//!
//! The crossing-detection lives in the **pure** [`wear_armor`] helper so it is
//! deterministic and unit-testable with no Bevy app; the caller (E3.6's
//! `apply_hit`, and the headless test here) writes the returned `Some` to a
//! [`bevy::prelude::MessageWriter<ArmorBroken>`] at the system boundary. Pure
//! model logic — no renderer, no pixel.

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

#[cfg(test)]
mod tests {
    use bevy::prelude::{App, Entity, MessageReader, MessageWriter, MinimalPlugins, Update, World};

    use super::*;
    use crate::armor::{
        ArmorFloor, ArmorHardness, ArmorIntegrity, ArmorPiece, ArmorProtection, ArmorType,
        BodyPart, SourceArmor, WornArmor,
    };

    /// A uniform worn suit at a chosen starting integrity — an arbitrary
    /// (NOT-shipped-tuning) magnitude, so the tests pin the wear *mechanism*, never
    /// a tuned value. The other three stats are irrelevant to wear and set to 0.
    fn worn_suit(integrity: i32) -> WornArmor {
        WornArmor::seed_from(&SourceArmor::uniform(ArmorPiece::new(
            ArmorFloor::new(0),
            ArmorProtection::new(0),
            ArmorIntegrity::new(integrity),
            ArmorHardness::new(0),
            ArmorType::DEFAULT,
        )))
    }

    /// A real, valid [`Entity`] id to stand in for the owning ganger — spawned from
    /// a throwaway [`World`] so the tests never hand-craft a raw id (0.18's
    /// `from_raw_u32` is fallible; spawning yields a guaranteed-valid handle without
    /// any `unwrap`).
    fn a_ganger() -> Entity {
        World::new().spawn_empty().id()
    }

    /// AC1 — a hit whose wear drives a piece's integrity to `≤ 0` leaves the worn
    /// copy at `≤ 0` and [`WornArmor::protects`] false thereafter (bare flesh).
    /// Drives the real [`wear_armor`] path past zero, then asserts the next read is
    /// unprotected.
    #[test]
    fn wear_past_zero_leaves_piece_unprotected() {
        let mut worn = worn_suit(3);
        let ganger = a_ganger();

        let _broke = wear_armor(&mut worn, BodyPart::Torso, IntegrityWear::new(10), ganger);

        assert!(
            *worn.at(BodyPart::Torso).integrity <= 0,
            "wear past the starting value must leave integrity ≤ 0",
        );
        assert!(
            !worn.protects(BodyPart::Torso),
            "a piece worn to ≤ 0 must stop protecting (bare flesh)",
        );
    }

    /// AC2 — the worn copy is battle-local: wearing it past zero never mutates the
    /// roster [`SourceArmor`] it was seeded from. Leans on the same isolation
    /// guarantee `armor::tests::wearing_worn_integrity_does_not_mutate_source`
    /// proves, but through the [`wear_armor`] entry point.
    #[test]
    fn wearing_does_not_mutate_source() {
        let source = SourceArmor::uniform(ArmorPiece::new(
            ArmorFloor::new(1),
            ArmorProtection::new(2),
            ArmorIntegrity::new(4),
            ArmorHardness::new(3),
            ArmorType::Ceramic,
        ));
        // SourceArmor is Copy — snapshot it by value, then prove the snapshot still
        // equals the source after the worn copy is worn past zero.
        let source_before = source;
        let mut worn = WornArmor::seed_from(&source);
        let ganger = a_ganger();

        // Wear the head's piece well past zero through the real entry point.
        let _broke = wear_armor(&mut worn, BodyPart::Head, IntegrityWear::new(100), ganger);

        // The roster source is unchanged on every location (the worn copy can never
        // reach it — battle-local seeding by value).
        assert_eq!(
            source, source_before,
            "the roster source must NOT change when the worn copy wears (whole record)",
        );
        for part in BodyPart::ALL {
            assert_eq!(
                source.at(part),
                source_before.at(part),
                "the roster source must NOT change when the worn copy wears at {part:?}",
            );
        }
    }

    /// AC3 (mechanism) — exactly one crossing: two successive [`wear_armor`] calls
    /// drive the SAME part past zero. The FIRST (protecting→broken) returns
    /// `Some(ArmorBroken)`; the SECOND (already broken) returns `None` — the signal
    /// fires once, never re-emitting on an already-broken piece.
    #[test]
    fn emits_exactly_once_on_the_crossing() {
        let mut worn = worn_suit(5);
        let ganger = a_ganger();

        // First hit crosses 5 → -1 (protecting → broken): emits.
        let first = wear_armor(&mut worn, BodyPart::LeftArm, IntegrityWear::new(6), ganger);
        assert_eq!(
            first,
            Some(ArmorBroken::new(ganger, BodyPart::LeftArm)),
            "the protecting→broken crossing must emit ArmorBroken once",
        );

        // Second hit re-wears an already-broken piece (-1 → -7): no re-emit.
        let second = wear_armor(&mut worn, BodyPart::LeftArm, IntegrityWear::new(6), ganger);
        assert_eq!(
            second, None,
            "an already-broken piece that re-wears must NOT re-emit (emit once)",
        );
    }

    /// A wear that does NOT cross zero (stays protecting) emits nothing — the
    /// signal is only the crossing, not every hit.
    #[test]
    fn no_emit_when_still_protecting() {
        let mut worn = worn_suit(10);
        let ganger = a_ganger();

        let result = wear_armor(&mut worn, BodyPart::RightLeg, IntegrityWear::new(4), ganger);

        assert_eq!(
            result, None,
            "a hit that leaves integrity > 0 must not emit ArmorBroken",
        );
        assert!(
            worn.protects(BodyPart::RightLeg),
            "the piece must still protect after a sub-fatal wear",
        );
    }

    /// AC4 (HEADLESS Bevy, `bevy-traps.md` #4) — [`ArmorBroken`] is a buffered
    /// `#[derive(Message)]`, NOT the observer `Event` API. Builds a headless app
    /// (`MinimalPlugins`, no window/renderer), registers the [`ArmorBroken`]
    /// message buffer, runs a one-shot system that calls [`wear_armor`] and writes
    /// the `Some` to a [`MessageWriter<ArmorBroken>`], `update()`s once, then a
    /// reader system drains the buffer into a resource the test asserts on.
    ///
    /// The sim crate cannot depend on `gdtf_test_utils` (that would cycle through
    /// `gdtf_app` → `gdtf_battle_sim`), so this uses the bare-`App` + `MinimalPlugins`
    /// fallback the contract sanctions — the same pattern
    /// [`crate::occupancy_sync`]'s headless tests use.
    #[test]
    fn armor_broken_is_a_buffered_message_read_in_a_headless_app() {
        use bevy::prelude::{IntoScheduleConfigs, Resource};

        /// Captures the [`ArmorBroken`] messages the reader system drained, so the
        /// test can assert on them after `update()` (no `unwrap` in the test body).
        #[derive(Resource, Default)]
        struct Captured(Vec<ArmorBroken>);

        let ganger = a_ganger();
        let part = BodyPart::Head;

        // The producer: wears a fresh worn suit past zero and writes the Some to the
        // buffered message. Owns its WornArmor locally — the sim's message boundary.
        let produce = move |mut writer: MessageWriter<ArmorBroken>| {
            let mut worn = WornArmor::seed_from(&SourceArmor::uniform(ArmorPiece::new(
                ArmorFloor::new(0),
                ArmorProtection::new(0),
                ArmorIntegrity::new(1),
                ArmorHardness::new(0),
                ArmorType::DEFAULT,
            )));
            if let Some(broke) = wear_armor(&mut worn, part, IntegrityWear::new(2), ganger) {
                writer.write(broke);
            }
        };

        // The consumer: drains the buffered messages (MessageReader, NOT an
        // observer) into the capture resource for assertion.
        let consume = |mut reader: MessageReader<ArmorBroken>,
                       mut captured: bevy::prelude::ResMut<Captured>| {
            for broke in reader.read() {
                captured.0.push(*broke);
            }
        };

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        // Register the buffered message — without this the MessageReader/Writer
        // params fail validation (the buffer must exist).
        app.add_message::<ArmorBroken>();
        app.init_resource::<Captured>();
        app.add_systems(Update, (produce, consume).chain());

        app.update();

        let captured = app
            .world()
            .get_resource::<Captured>()
            .map_or_else(Vec::new, |c| c.0.clone());

        assert_eq!(
            captured.len(),
            1,
            "exactly one ArmorBroken message must be read from the buffer",
        );
        assert_eq!(
            captured.first(),
            Some(&ArmorBroken::new(ganger, part)),
            "the buffered ArmorBroken must carry the right ganger Entity + BodyPart",
        );
    }
}
