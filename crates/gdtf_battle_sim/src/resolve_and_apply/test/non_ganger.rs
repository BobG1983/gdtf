use super::support::*;

/// AC4 — non-ganger SURFACE outcomes are inert on a ganger: each of Slab / Ground /
/// Miss yields a no-damage report and leaves the ganger's pools and state untouched
/// (and takes no draw). (`ShotKind::Cover` is NO LONGER inert as of GTW-364 — it
/// depletes cover HP through the same fold; its behavior is asserted in
/// [`super::cover`], so it is deliberately excluded from this inert sweep.)
#[test]
fn non_ganger_outcomes_are_inert() {
    let tuning = CombatTuning::default();
    let weapon = a_weapon(50, 50, 50, DamageType::Rend);

    // The non-ganger, non-cover kinds: a Slab / Ground carries a struck cell, a Miss
    // carries nothing. `resolve_and_apply`'s kind dispatch folds each of these to a
    // no-effect report (no draw, no mutation) — they strike neither a ganger nor cover.
    let kinds = [
        ShotKind::Miss,
        ShotKind::Slab(CellLevel::new(Cell::new(2, 2), Level::new(1))),
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
            &mut cover,
            &tuning,
            &mut rng_used,
        );

        assert_eq!(
            report.applied, None,
            "a {kind:?} outcome must apply no damage to a ganger",
        );
        assert_eq!(report.part, None, "a {kind:?} report carries no part");
        assert_eq!(report.kind, kind, "the report still names the struck kind");
        assert_eq!(
            report.cover_destroyed, None,
            "a {kind:?} (non-cover) outcome destroys no cover",
        );

        assert_eq!(hp, hp_before, "{kind:?} must not change Hp");
        assert_eq!(wounds, wounds_before, "{kind:?} must not change Wounds");
        assert_eq!(life, life_before, "{kind:?} must not change LifeState");
        assert_eq!(
            integrity, integrity_before,
            "{kind:?} must not wear the struck piece's integrity"
        );
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
