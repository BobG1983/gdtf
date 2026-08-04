use super::support::*;
use crate::resolve_and_apply::{ShotSource, WoundRoll};

#[test]
fn same_seed_reproduces_the_report_sequence() {
    let tuning = CombatTuning::default();
    let entity = an_entity();
    let weapon = a_weapon(16, 9, 4, DamageType::Blast);
    let parts = [
        BodyPart::Head,
        BodyPart::Torso,
        BodyPart::LeftArm,
        BodyPart::RightLeg,
        BodyPart::Torso,
    ];

    let run = || {
        let mut r = rng();
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
                let piece = integrity
                    .get_mut(&part)
                    .map(|integ| struck_piece(1, 6, 2, ArmorType::Flak, integ));
                resolve_and_apply(
                    &ganger_outcome(entity, part),
                    ShotSource {
                        weapon: weapon.stats(),
                        luck:   Luck::new(1.0),
                    },
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
                    &mut WoundRoll {
                        tuning:       &tuning,
                        severity_rng: &mut r,
                        tables:       &injury_tables(),
                        registry:     &injury_registry(),
                        injury_rng:   &mut injury_rng(),
                    },
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
