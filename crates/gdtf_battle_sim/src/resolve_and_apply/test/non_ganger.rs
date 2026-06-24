use super::support::*;

/// AC4 — the truly-inert non-ganger SURFACE outcomes leave a ganger untouched: each of
/// Ground / Miss yields a no-damage report and leaves the ganger's pools and state
/// untouched (and takes no draw). (`ShotKind::Cover` is NO LONGER inert as of GTW-364,
/// nor is `ShotKind::Slab` as of GTW-365 — both deplete their own structural HP through
/// the same fold; their behavior is asserted in [`super::cover`] / [`super::slab`], so
/// they are deliberately excluded from this inert sweep.)
#[test]
fn non_ganger_outcomes_are_inert() {
    let tuning = CombatTuning::default();
    let weapon = a_weapon(50, 50, 50, DamageType::Rend);

    // The truly-inert non-ganger kinds: a Ground carries a struck cell, a Miss carries
    // nothing. `resolve_and_apply`'s kind dispatch folds each of these to a no-effect
    // report (no draw, no mutation) — they strike neither a ganger nor a structural
    // surface with its own HP (Cover / Slab DO, so they are excluded above).
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
        assert_eq!(
            report.slab_destroyed, None,
            "a {kind:?} (non-slab) outcome destroys no slab",
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
