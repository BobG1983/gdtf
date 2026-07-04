//! Relocated unit tests for armor wear persistence (GTW-201 wave 22 — moved verbatim
//! from the former inline `#[cfg(test)] mod tests`).

use bevy::prelude::{App, Entity, MessageReader, MessageWriter, MinimalPlugins, Update, World};

use crate::{
    armor::{ArmorIntegrity, BodyPart},
    armor_wear::{ArmorBroken, ArmorDamaged, ArmorWearOutcome, wear_armor},
    resolve_hit::IntegrityWear,
};

/// A struck piece's [`ArmorIntegrity`] component at a chosen starting magnitude — an
/// arbitrary (NOT-shipped-tuning) value, so the tests pin the wear *mechanism*, never
/// a tuned value. Since GTW-323 (ADR-0004) [`wear_armor`] mutates the piece entity's
/// integrity component directly, so the unit tests drive that component by `&mut`.
fn piece_integrity(integrity: i32) -> ArmorIntegrity {
    ArmorIntegrity::new(integrity)
}

/// A real, valid [`Entity`] id to stand in for the owning ganger — spawned from
/// a throwaway [`World`] so the tests never hand-craft a raw id (0.18's
/// `from_raw_u32` is fallible; spawning yields a guaranteed-valid handle without
/// any `unwrap`).
fn a_ganger() -> Entity {
    World::new().spawn_empty().id()
}

/// AC1 — a hit whose wear drives a piece's integrity to `≤ 0` leaves the piece
/// integrity at `≤ 0` (no longer protecting — bare flesh thereafter). Drives the real
/// [`wear_armor`] path past zero, then asserts the resulting component is `≤ 0`.
#[test]
fn wear_past_zero_leaves_piece_unprotected() {
    let mut integrity = piece_integrity(3);
    let ganger = a_ganger();

    let _broke = wear_armor(
        &mut integrity,
        BodyPart::Torso,
        IntegrityWear::new(10),
        ganger,
    );

    assert!(
        *integrity <= 0,
        "wear past the starting value must leave integrity ≤ 0 (bare flesh thereafter)",
    );
}

/// AC3 (mechanism) — exactly one crossing: two successive [`wear_armor`] calls
/// drive the SAME piece past zero. The FIRST (protecting→broken) returns
/// `Broke(ArmorBroken)`; the SECOND (already broken) returns `Unaffected` — the signal
/// fires once, never re-emitting on an already-broken piece.
#[test]
fn emits_exactly_once_on_the_crossing() {
    let mut integrity = piece_integrity(5);
    let ganger = a_ganger();

    // First hit crosses 5 → -1 (protecting → broken): emits Broke once.
    let first = wear_armor(
        &mut integrity,
        BodyPart::LeftArm,
        IntegrityWear::new(6),
        ganger,
    );
    assert_eq!(
        first,
        ArmorWearOutcome::Broke(ArmorBroken::new(ganger, BodyPart::LeftArm)),
        "the protecting→broken crossing must emit Broke(ArmorBroken) once",
    );

    // Second hit re-wears an already-broken piece (-1 → -7): no re-emit (Unaffected,
    // NOT Damaged — an already-broken piece is not protecting, so it surfaces nothing).
    let second = wear_armor(
        &mut integrity,
        BodyPart::LeftArm,
        IntegrityWear::new(6),
        ganger,
    );
    assert_eq!(
        second,
        ArmorWearOutcome::Unaffected,
        "an already-broken piece that re-wears must NOT re-emit (emit once)",
    );
}

/// A wear that does NOT cross zero (stays protecting) emits no [`ArmorBroken`] — but
/// it DOES surface the GTW-313 [`ArmorDamaged`] reduction carrying the exact delta (the
/// wear signal is per reduction, the break signal is only the crossing).
#[test]
fn worn_not_broken_when_still_protecting() {
    let mut integrity = piece_integrity(10);
    let ganger = a_ganger();

    let result = wear_armor(
        &mut integrity,
        BodyPart::RightLeg,
        IntegrityWear::new(4),
        ganger,
    );

    assert_eq!(
        result,
        ArmorWearOutcome::Damaged(ArmorDamaged::new(
            ganger,
            BodyPart::RightLeg,
            IntegrityWear::new(4)
        )),
        "a hit that leaves integrity > 0 must surface Damaged(delta=4), never Broke",
    );
    assert!(
        *integrity > 0,
        "the piece must still protect (integrity > 0) after a sub-fatal wear",
    );
}

/// A ZERO-wear hit on a still-protecting piece surfaces NEITHER signal — Unaffected
/// (no "Armor -0" pop, mirroring [`ArmorBroken`]'s emit-only-on-the-event rule).
#[test]
fn zero_wear_on_protecting_piece_is_unaffected() {
    let mut integrity = piece_integrity(10);
    let ganger = a_ganger();

    let result = wear_armor(
        &mut integrity,
        BodyPart::RightLeg,
        IntegrityWear::new(0),
        ganger,
    );

    assert_eq!(
        result,
        ArmorWearOutcome::Unaffected,
        "a zero-wear hit must surface nothing (no misleading Armor -0)",
    );
    assert!(
        *integrity > 0,
        "a zero-wear hit must leave the piece protecting (integrity > 0)",
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

    // The producer: wears a fresh piece integrity past zero and writes the payload to
    // the buffered message. Owns its piece integrity locally — the sim's message
    // boundary (since GTW-323 `wear_armor` mutates the piece component directly).
    let produce = move |mut writer: MessageWriter<ArmorBroken>| {
        let mut integrity = piece_integrity(1);
        // The breaking wear yields Broke(ArmorBroken) — write its payload to the buffer
        // (a Damaged/Unaffected outcome would write nothing, as the old `if let Some` did).
        if let ArmorWearOutcome::Broke(broke) =
            wear_armor(&mut integrity, part, IntegrityWear::new(2), ganger)
        {
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
