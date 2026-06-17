//! Relocated unit tests for armor wear persistence (GTW-201 wave 22 — moved verbatim
//! from the former inline `#[cfg(test)] mod tests`).

use bevy::prelude::{App, Entity, MessageReader, MessageWriter, MinimalPlugins, Update, World};

use crate::{
    armor::{
        ArmorFloor, ArmorHardness, ArmorIntegrity, ArmorPiece, ArmorProtection, ArmorType,
        BodyPart, SourceArmor, WornArmor,
    },
    armor_wear::{ArmorBroken, wear_armor},
    resolve_hit::IntegrityWear,
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
