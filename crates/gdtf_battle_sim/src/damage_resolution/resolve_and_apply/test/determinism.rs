use super::support::*;

/// AC5 — seeded determinism: two `resolve_and_apply` runs from the same
/// `BattleSeed`, over the same outcome SEQUENCE on identical fresh targets,
/// produce identical report sequences (replay equality). Walks a sequence so the
/// stream — not just a single draw — is reproduced.
#[test]
fn same_seed_reproduces_the_report_sequence() {
    let tuning = CombatTuning::default();
    let entity = an_entity();
    let weapon = a_weapon(16, 9, 4, DamageType::Blast);
    // A sequence of struck parts so the stream is genuinely walked across calls.
    let parts = [
        BodyPart::Head,
        BodyPart::Torso,
        BodyPart::LeftArm,
        BodyPart::RightLeg,
        BodyPart::Torso,
    ];

    let run = || {
        let mut r = SimRng::from_seed(BattleSeed::new(SEED));
        // A fresh target per call so wear/state don't drift the comparison. Each part
        // has its own piece-entity integrity (keyed access — GTW-323 / ADR-0004), so the
        // repeated Torso hit wears the SAME piece across the sequence (the same
        // per-location wear the old shared uniform suit modelled).
        let mut hp = Hp::new(60);
        let mut wounds = Wounds::new(12);
        let mut life = LifeState::Alive;
        let mut integrity: bevy::platform::collections::HashMap<BodyPart, ArmorIntegrity> =
            BodyPart::ALL
                .into_iter()
                .map(|p| (p, piece_integrity(40)))
                .collect();
        let mut inflicted = InflictedWounds::default();
        parts
            .iter()
            .map(|&part| {
                // Key the struck part's piece integrity (`ganger → Wears → the
                // BodyPart-tagged piece` in the full ECS path; a per-part map here).
                let piece = integrity
                    .get_mut(&part)
                    .map(|integ| struck_piece(1, 6, 2, ArmorType::Flak, integ));
                resolve_and_apply(
                    &ganger_outcome(entity, part),
                    weapon.stats(),
                    Luck::new(1.0),
                    Some(TargetGanger {
                        hp: &mut hp,
                        wounds: &mut wounds,
                        life: &mut life,
                        piece,
                        inflicted: &mut inflicted,
                        toughness: Toughness::new(2.0),
                        luck: Luck::new(2.0),
                    }),
                    entity,
                    surfaces(&mut ledger(), &mut slab_ledger()),
                    &tuning,
                    &mut r,
                )
            })
            .collect::<Vec<_>>()
    };

    assert_eq!(
        run(),
        run(),
        "the same battle seed must reproduce the identical report sequence",
    );
}
