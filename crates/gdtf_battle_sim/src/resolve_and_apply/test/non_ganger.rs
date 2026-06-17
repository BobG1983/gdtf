use super::support::*;

/// AC4 — non-ganger outcomes are inert on a ganger: each of Cover / Slab /
/// Ground / Miss yields a no-damage report and leaves the ganger's pools and
/// state untouched (and takes no draw).
#[test]
fn non_ganger_outcomes_are_inert() {
    let tuning = CombatTuning::default();
    let weapon = a_weapon(50, 50, 50, DamageType::Rend);

    // Each non-ganger kind. AC4 enumerates Cover / Slab / Ground / Miss
    // explicitly, so all four are swept here. Cover carries a struck
    // CoverEntry fixture (seeded full-HP); Slab/Ground carry a struck cell;
    // Miss carries nothing. E3.9's early-return branches only on the variant
    // (`let ShotKind::Ganger(_) = ... else`), so every one must be inert.
    let kinds = [
        ShotKind::Miss,
        ShotKind::Cover(CoverEntry::seeded(
            CoverHp::new(40),
            HeightBand::Mid,
            ArmorProtection::new(3),
            ArmorHardness::new(2),
        )),
        ShotKind::Slab(CellLevel::new(Cell::new(2, 2), Level::new(1))),
        ShotKind::Ground(CellLevel::new(Cell::new(2, 2), Level::new(0))),
    ];

    for kind in kinds {
        let mut hp = Hp::new(30);
        let mut wounds = Wounds::new(5);
        let mut life = LifeState::Alive;
        let mut worn = worn_suit(0, 0, 1, 0, ArmorType::DEFAULT);
        let mut inflicted = InflictedWounds::default();

        let hp_before = hp;
        let wounds_before = wounds;
        let life_before = life;
        let worn_before = worn;

        let mut rng_used = rng();
        let report = resolve_and_apply(
            &non_ganger_outcome(kind),
            weapon.stats(),
            Luck::new(3.0),
            TargetGanger {
                hp:        &mut hp,
                wounds:    &mut wounds,
                life:      &mut life,
                worn:      &mut worn,
                inflicted: &mut inflicted,
                toughness: Toughness::new(1.0),
                luck:      Luck::new(1.0),
            },
            an_entity(),
            &tuning,
            &mut rng_used,
        );

        assert_eq!(
            report.applied, None,
            "a {kind:?} outcome must apply no damage to a ganger",
        );
        assert_eq!(report.part, None, "a {kind:?} report carries no part");
        assert_eq!(report.kind, kind, "the report still names the struck kind");

        assert_eq!(hp, hp_before, "{kind:?} must not change Hp");
        assert_eq!(wounds, wounds_before, "{kind:?} must not change Wounds");
        assert_eq!(life, life_before, "{kind:?} must not change LifeState");
        assert_eq!(worn, worn_before, "{kind:?} must not wear WornArmor");
        assert!(
            inflicted.is_empty(),
            "{kind:?} (non-ganger) must record NO InflictedWound (GTW-279)",
        );

        // No draw taken on a non-ganger outcome.
        let mut rng_fresh = rng();
        assert_eq!(
            rng_used.next_u64(),
            rng_fresh.next_u64(),
            "a {kind:?} outcome must take NO draw",
        );
    }
}
