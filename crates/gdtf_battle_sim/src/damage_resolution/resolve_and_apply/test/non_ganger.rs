use super::support::*;

#[test]
fn non_ganger_outcomes_are_inert() {
    let tuning = CombatTuning::default();
    let weapon = a_weapon(50, 50, 50, DamageType::Rend);

    let kinds = [
        ShotKind::Miss,
        ShotKind::Ground(CellLevel::new(Cell::new(2, 2), Level::new(0))),
    ];

    for kind in kinds {
        let mut hp = Hp::new(30);
        let mut wounds = Wounds::new(5);
        let mut life = LifeState::Alive;
        let mut integrity = piece_integrity(1);
        let mut inflicted = InflictedWounds::default();
        let mut cover = ledger();

        let hp_before = hp;
        let wounds_before = wounds;
        let life_before = life;
        let integrity_before = integrity;

        let mut rng_used = rng();
        let report = resolve_and_apply(
            &non_ganger_outcome(kind),
            weapon.stats(),
            Luck::new(3.0),
            Some(TargetGanger {
                hp:        &mut hp,
                wounds:    &mut wounds,
                life:      &mut life,
                piece:     Some(struck_piece(0, 0, 0, ArmorType::DEFAULT, &mut integrity)),
                inflicted: &mut inflicted,
                toughness: Toughness::new(1.0),
                luck:      Luck::new(1.0),
            }),
            an_entity(),
            surfaces(&mut cover, &mut slab_ledger()),
            &tuning,
            &mut rng_used,
            &injury_tables(),
            &injury_registry(),
            &mut injury_rng(),
        );

        assert!(
            !matches!(report.verdict, HitVerdict::Ganger(_)),
            "a {kind:?} outcome must apply no damage to a ganger",
        );
        assert!(
            !matches!(report.verdict, HitVerdict::Cover(_) | HitVerdict::Slab(_)),
            "a {kind:?} (non-cover, non-slab) outcome touches no structural ledger",
        );
        assert_eq!(report.kind, kind, "the report still names the struck kind");

        assert_eq!(hp, hp_before, "{kind:?} must not change Hp");
        assert_eq!(wounds, wounds_before, "{kind:?} must not change Wounds");
        assert_eq!(life, life_before, "{kind:?} must not change LifeState");
        assert_eq!(
            integrity, integrity_before,
            "{kind:?} must not wear the struck piece's integrity"
        );
        assert!(
            inflicted.is_empty(),
            "{kind:?} (non-ganger) must record NO InflictedWound ",
        );

        let mut rng_fresh = rng();
        assert_eq!(
            rng_used.next_u64(),
            rng_fresh.next_u64(),
            "a {kind:?} outcome must take NO draw",
        );
    }
}
