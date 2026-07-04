//! Armor wear through the fold (AC7) + the per-tier `wound_cost` mapping
//! (no-bare-types): `apply_hit` drops the struck piece's integrity by the hit's
//! wear, surfaces the [`ArmorBroken`] signal through a message buffer, and the
//! [`wound_cost`] helper maps each severity tier to its tunable cost.

use super::support::*;

/// AC7 (wear path) — `apply_hit` wears the struck piece as part of application: the
/// struck location's integrity drops by exactly the hit's wear, a sub-fatal wear on
/// a still-protecting piece returns `Damaged` carrying that exact delta (GTW-313), and
/// a high-wear hit on a near-broken piece returns `Broke(ArmorBroken)`. A unit
/// assertion on the worn copy + the per-hit outcome through the real `apply_hit`.
#[test]
fn apply_hit_wears_the_struck_piece_and_can_break_it() {
    let tuning = CombatTuning::default();
    let part = BodyPart::RightArm;

    // (1) Wears by exactly the hit's wear: a sturdy piece, sub-fatal wear. The piece
    //     stays protecting, so apply_hit returns Damaged carrying the exact delta (6).
    let ganger1 = a_ganger();
    let mut hp = Hp::new(50);
    let mut wounds = Wounds::new(9);
    let mut life = LifeState::Alive;
    let mut integrity = worn_piece_integrity(20);
    let mut inflicted = InflictedWounds::default();
    let before = *integrity;
    {
        let target = GangerHitTarget {
            hp:        &mut hp,
            wounds:    &mut wounds,
            life:      &mut life,
            integrity: Some(&mut integrity),
            inflicted: &mut inflicted,
        };
        let outcome = apply_hit(target, &hit(1, 6), Severity::Minor, part, ganger1, &tuning);
        assert_eq!(
            outcome,
            ArmorWearOutcome::Damaged(ArmorDamaged::new(ganger1, part, IntegrityWear::new(6))),
            "a sub-fatal wear on a sturdy piece must not break it — it must report Damaged(delta=6)",
        );
    }
    assert_eq!(
        *integrity,
        before - 6,
        "apply_hit must drop the struck piece entity's integrity by exactly the hit's wear",
    );

    // (2) A high-wear hit on a near-broken piece returns Broke(ArmorBroken).
    let ganger = a_ganger();
    let mut hp2 = Hp::new(50);
    let mut wounds2 = Wounds::new(9);
    let mut life2 = LifeState::Alive;
    let mut integrity2 = worn_piece_integrity(1); // protecting (1 > 0), one hit from broken
    let mut inflicted2 = InflictedWounds::default();
    let outcome = {
        let target = GangerHitTarget {
            hp:        &mut hp2,
            wounds:    &mut wounds2,
            life:      &mut life2,
            integrity: Some(&mut integrity2),
            inflicted: &mut inflicted2,
        };
        apply_hit(target, &hit(1, 5), Severity::Minor, part, ganger, &tuning)
    };
    assert_eq!(
        outcome,
        ArmorWearOutcome::Broke(ArmorBroken::new(ganger, part)),
        "a high-wear hit crossing a near-broken piece to ≤ 0 must return Broke(ArmorBroken)",
    );
    assert!(
        *integrity2 <= 0,
        "the struck piece must be broken (≤ 0) after the crossing hit",
    );
}

/// AC7 (HEADLESS, `bevy-traps.md` #4) — the [`ArmorBroken`] `apply_hit` surfaces is
/// a buffered `#[derive(Message)]`, written through a real `MessageWriter` at the
/// system boundary. A `MinimalPlugins` app runs a one-shot system that builds a
/// near-broken ganger, calls `apply_hit`, and writes the returned `Some` to the
/// buffer; a reader drains it into a capture resource the test asserts on. The
/// same bare-App + `add_message` pattern `armor_wear.rs` sanctions (the sim crate
/// cannot depend on `gdtf_test_utils`).
#[test]
fn apply_hit_armor_broken_flows_through_a_message_buffer() {
    use bevy::prelude::{IntoScheduleConfigs, MessageReader, MessageWriter, ResMut, Resource};

    /// Captures the drained [`ArmorBroken`] messages for assertion after
    /// `update()` (no `unwrap` in the test body).
    #[derive(Resource, Default)]
    struct Captured(Vec<ArmorBroken>);

    let ganger = a_ganger();
    let part = BodyPart::Torso;

    // Producer: builds a near-broken ganger locally, applies a breaking hit
    // through the real apply_hit, and writes the Some to the buffer — the sim's
    // message boundary.
    let produce = move |mut writer: MessageWriter<ArmorBroken>| {
        let tuning = CombatTuning::default();
        let mut hp = Hp::new(30);
        let mut wounds = Wounds::new(6);
        let mut life = LifeState::Alive;
        let mut integrity = worn_piece_integrity(1);
        let mut inflicted = InflictedWounds::default();
        let target = GangerHitTarget {
            hp:        &mut hp,
            wounds:    &mut wounds,
            life:      &mut life,
            integrity: Some(&mut integrity),
            inflicted: &mut inflicted,
        };
        // The breaking hit yields Broke(ArmorBroken) — write its payload to the buffer
        // (the system-boundary message write; a Damaged/Unaffected outcome would write
        // nothing here, exactly as the old `if let Some` did).
        if let ArmorWearOutcome::Broke(broke) =
            apply_hit(target, &hit(1, 5), Severity::Minor, part, ganger, &tuning)
        {
            writer.write(broke);
        }
    };

    // Consumer: drains the buffered messages (MessageReader, NOT an observer).
    let consume = |mut reader: MessageReader<ArmorBroken>, mut captured: ResMut<Captured>| {
        for broke in reader.read() {
            captured.0.push(*broke);
        }
    };

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
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
        "apply_hit's ArmorBroken must reach the message buffer once"
    );
    assert_eq!(
        captured.first(),
        Some(&ArmorBroken::new(ganger, part)),
        "the buffered ArmorBroken must carry the struck ganger Entity + BodyPart",
    );
}

/// No-bare-types / mechanism — the per-tier [`WoundCost`] newtype derefs to its
/// inner `u8`, and [`wound_cost`] maps each severity tier to its tunable cost:
/// None (and the structurally-handled Fatal) cost 0; Minor/Major/Critical read
/// the tuning. Asserts the ordering relation, never the shipped split.
#[test]
fn wound_cost_helper_maps_tiers_and_newtype_derefs() {
    let costs = WoundCosts::default();
    // None is a true zero spend; Fatal is handled structurally so its helper
    // value is the floor 0 (apply_hit never reaches it for Fatal).
    assert_eq!(*wound_cost(Severity::None, costs), 0);
    assert_eq!(*wound_cost(Severity::Fatal, costs), 0);
    // The three middle tiers read the tunable costs, ascending — a relation,
    // never a pinned magnitude.
    let minor = *wound_cost(Severity::Minor, costs);
    let major = *wound_cost(Severity::Major, costs);
    let critical = *wound_cost(Severity::Critical, costs);
    assert!(
        minor < major,
        "Minor cost must be < Major: {minor} >= {major}"
    );
    assert!(
        major < critical,
        "Major cost must be < Critical: {major} >= {critical}"
    );
    // The newtype derefs to its inner u8 (arbitrary value, mechanism not value).
    assert_eq!(*WoundCost::new(7), 7u8);
}
