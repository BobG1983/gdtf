//! GTW-438 — `roll_injury` draw-discipline + determinism.

use super::{
    super::{InjuryRegistry, InjuryTables, roll_injury},
    support::*,
};
use crate::{armor::BodyPart, severity::Severity};

#[test]
fn roll_injury_takes_no_draw_for_none_or_fatal() {
    // C1/C2 + stream-alignment (#2): a graze (None) and a Fatal take NO InjuryRng draw and
    // return None. Proof: the rng cursor is UNMOVED — its next raw u64 equals a fresh
    // stream's next raw u64.
    let (registry, tables) = one_injury_table("hurt", BodyPart::Head, Severity::Minor);
    for severity in [Severity::None, Severity::Fatal] {
        let mut rng = injury_rng();
        let rolled = roll_injury(BodyPart::Head, severity, &tables, &registry, &mut rng);
        assert!(
            rolled.is_none(),
            "{severity:?} is not tabled — no injury rolled"
        );
        // The cursor did NOT advance: next draw equals a fresh stream's first draw.
        assert_eq!(
            rng.next_u64(),
            injury_rng().next_u64(),
            "{severity:?} must take NO InjuryRng draw (the cursor stays put)"
        );
    }
}

#[test]
fn roll_injury_draws_exactly_one_and_resolves_a_tabled_severity() {
    // C1: a Minor/Major/Critical wound with content takes EXACTLY ONE draw and resolves
    // the picked key to its InjuryDef.
    let (registry, tables) = one_injury_table("hurt", BodyPart::Torso, Severity::Major);
    let mut rng = injury_rng();
    let rolled = roll_injury(
        BodyPart::Torso,
        Severity::Major,
        &tables,
        &registry,
        &mut rng,
    );
    assert!(
        rolled.is_some(),
        "a populated Major bucket must roll an injury"
    );
    let Some(rolled) = rolled else {
        return;
    };
    assert_eq!(rolled.part, BodyPart::Torso);
    assert_eq!(rolled.severity, Severity::Major);
    assert_eq!(
        rolled.effects.len(),
        1,
        "the def's single effect is frozen on"
    );

    // EXACTLY ONE draw: the cursor advanced by one sample. A fresh stream that takes ONE
    // throwaway draw then matches this rng's next draw.
    let mut fresh = injury_rng();
    let _ = fresh.next_u64(); // one draw (the roll's)
    assert_eq!(
        rng.next_u64(),
        fresh.next_u64(),
        "a tabled roll must take EXACTLY ONE InjuryRng draw"
    );
}

#[test]
fn roll_injury_empty_table_draws_then_discards_content_independent() {
    // C1 + STREAM-ALIGNMENT (#2): the KEY content-independence property. A Minor wound on
    // an EMPTY/MISSING bucket STILL takes its one draw (then discards → None), so the
    // InjuryRng cursor ends at the SAME position whether the bucket has content or is
    // empty — a content hot-edit cannot desync replay.
    let empty_registry = InjuryRegistry::default();
    let empty_tables = InjuryTables::default();
    let (full_registry, full_tables) = one_injury_table("hurt", BodyPart::Head, Severity::Minor);

    // Drive BOTH scenarios from an identical fresh stream.
    let mut rng_empty = injury_rng();
    let empty = roll_injury(
        BodyPart::Head,
        Severity::Minor,
        &empty_tables,
        &empty_registry,
        &mut rng_empty,
    );
    assert!(empty.is_none(), "an empty bucket rolls no injury");

    let mut rng_full = injury_rng();
    let full = roll_injury(
        BodyPart::Head,
        Severity::Minor,
        &full_tables,
        &full_registry,
        &mut rng_full,
    );
    assert!(full.is_some(), "a populated bucket rolls an injury");

    // CONTENT-INDEPENDENCE: after one Minor roll each, BOTH cursors are at the SAME
    // position (each took EXACTLY ONE draw), regardless of whether content existed. The
    // post-draw streams produce identical next values.
    assert_eq!(
        rng_empty.next_u64(),
        rng_full.next_u64(),
        "the InjuryRng cursor must end at the SAME position for an empty vs a populated \
         bucket — the one draw is content-independent (stream alignment)"
    );
}

#[test]
fn roll_injury_is_deterministic_for_a_fixed_seed() {
    // C1: pure given the rng state — the same seed reproduces the same pick. (Determinism
    // at the roll level; the seeded-replay E2E lives in the acts injury test.)
    let (registry, tables) = one_injury_table("hurt", BodyPart::LeftLeg, Severity::Critical);
    let mut rng_a = injury_rng();
    let mut rng_b = injury_rng();
    let a = roll_injury(
        BodyPart::LeftLeg,
        Severity::Critical,
        &tables,
        &registry,
        &mut rng_a,
    );
    let b = roll_injury(
        BodyPart::LeftLeg,
        Severity::Critical,
        &tables,
        &registry,
        &mut rng_b,
    );
    assert_eq!(a, b, "the same seed must reproduce the same rolled injury");
}
