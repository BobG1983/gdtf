//! GTW-641 — the Downed-ENTRY gate on the §9 Wounds bleed-out: a ganger downed BY
//! this tick's injury-HP bleed has been down ZERO rounds, so THAT tick drains no
//! bleed-out Wound (and can never chain Alive→Downed→Dead within one tick). Only a
//! ganger who ENTERED the tick already Downed drains — which makes the weapon
//! down-path (downed mid-turn by [`apply_hit`]'s terminal gate) and the
//! injury-HP-bleed down-path share the SAME first-Downed-tick Wound behavior.
//!
//! All three tests drive the REAL production path — the seeded live-runtime
//! harness ([`super::support::live_app`]: `BattleSimPlugin`, turn cycle, the
//! `enemy_phase_started`-gated `tick_bleed` registration), not a hand-rolled tick.

use super::support::{PLAYER, bleed_rate, bleeding_ganger, end_turn, life_of, live_app, wounds_of};
use crate::{
    apply_hit::{GangerHitTarget, apply_hit},
    armor::BodyPart,
    ganger::{Hp, LifeState, Wounds},
    inflicted_wound::InflictedWounds,
    injuries::BleedAfflicted,
    resolve_hit::{HitResult, HpDamage, IntegrityWear, PenetratingDamage},
    severity::Severity,
    tuning::CombatTuning,
};

/// An arbitrary (non-tuned) injury-bleed accrual == starting Hp, so ONE injury-HP
/// bleed tick empties the pool exactly and downs the ganger.
const ACCRUAL: u16 = 4;

/// GTW-641 (C1) — a ganger whose injury-HP bleed downs them THIS tick ends the tick
/// [`LifeState::Downed`] with their §9 bleed-out Wound pool UNTOUCHED: a just-downed
/// ganger has been down zero rounds, so the same tick must not also drain a Wound.
#[test]
fn a_ganger_downed_by_the_injury_bleed_keeps_its_wounds_that_round() {
    let rate = bleed_rate();
    // A pool well above the rate so a (wrong) same-tick drain would be measurable.
    let start = rate.saturating_add(10);

    let mut app = live_app();
    let ganger = bleeding_ganger(&mut app, PLAYER, LifeState::Alive, start);
    // The GTW-438 injury-HP bleed inputs: an Hp pool the accrued bleed empties in
    // exactly one tick (accrual == Hp — arbitrary test data, not shipped tuning).
    app.world_mut()
        .entity_mut(ganger)
        .insert((Hp::new(ACCRUAL), BleedAfflicted::new(ACCRUAL)));

    end_turn(&mut app); // one full round — the tick both bleeds Hp and downs them

    assert_eq!(
        life_of(&app, ganger),
        LifeState::Downed,
        "the injury-HP bleed emptying the Hp pool must DOWN the ganger (never kill)",
    );
    assert_eq!(
        wounds_of(&app, ganger),
        start,
        "a ganger downed BY this tick has been down ZERO rounds — its §9 bleed-out \
         Wound pool must be UNTOUCHED that tick (GTW-641)",
    );
}

/// GTW-641 (C1) — the lethal chain is impossible: a ganger one Wound-round from death
/// who is downed by THIS tick's injury-HP bleed must end the tick Downed, never
/// Alive→Downed→Dead within the single tick.
#[test]
fn a_ganger_downed_by_the_injury_bleed_cannot_die_the_same_tick() {
    let rate = bleed_rate();
    // EXACTLY one bleed-out round in the pool — a (wrong) same-tick Wounds drain
    // would empty it and kill through the terminal gate.
    let start = rate;

    let mut app = live_app();
    let ganger = bleeding_ganger(&mut app, PLAYER, LifeState::Alive, start);
    app.world_mut()
        .entity_mut(ganger)
        .insert((Hp::new(ACCRUAL), BleedAfflicted::new(ACCRUAL)));

    end_turn(&mut app);

    assert_eq!(
        life_of(&app, ganger),
        LifeState::Downed,
        "Alive→Downed→Dead within ONE tick must be impossible — the tick that downs \
         drains no Wound, so the terminal gate cannot fire the same tick (GTW-641)",
    );
    assert_eq!(
        wounds_of(&app, ganger),
        start,
        "the downing tick leaves the one-round Wound pool intact",
    );
}

/// Down `ganger` mid-turn through the REAL weapon fold — [`apply_hit`] with a graze
/// ([`Severity::None`] spends no Wounds) whose HP loss overshoots the pool, tripping
/// the Hp→0→Downed terminal gate — then persist the folded pools onto the entity.
fn weapon_down(app: &mut bevy::prelude::App, ganger: bevy::prelude::Entity, start_wounds: u8) {
    let mut hp = Hp::new(ACCRUAL);
    let mut wounds = Wounds::new(start_wounds);
    let mut life = LifeState::Alive;
    let mut inflicted = InflictedWounds::default();
    let tuning = CombatTuning::default();
    // An overshooting HP loss (arbitrary magnitude) — bare flesh (no worn piece).
    let hit = HitResult {
        penetrating: PenetratingDamage::new(20),
        hp_damage:   HpDamage::new(20),
        wear:        IntegrityWear::new(0),
    };
    let _wear = apply_hit(
        GangerHitTarget {
            hp:        &mut hp,
            wounds:    &mut wounds,
            life:      &mut life,
            integrity: None,
            inflicted: &mut inflicted,
        },
        &hit,
        Severity::None,
        BodyPart::Torso,
        ganger,
        &tuning,
    );
    assert_eq!(life, LifeState::Downed, "the graze's HP loss must down");
    assert_eq!(*wounds, start_wounds, "a graze spends no Wounds");
    app.world_mut()
        .entity_mut(ganger)
        .insert((hp, wounds, life));
}

/// GTW-641 (A2) — down-path parity: a WEAPON-downed ganger (downed mid-turn, before
/// the round's bleed tick) and an INJURY-HP-BLEED-downed ganger (downed BY the round's
/// bleed tick) show the SAME first-Downed-tick Wound behavior — the first tick each
/// ENTERS already Downed drains exactly one `bleed_rate`, and the tick that CAUSES a
/// down drains nothing.
#[test]
fn weapon_and_injury_downs_share_the_first_downed_tick_wound_behavior() {
    let rate = bleed_rate();
    let start = rate.saturating_mul(2).saturating_add(10);

    let mut app = live_app();
    // The weapon path: downed mid-turn (before any turn boundary) via apply_hit.
    let by_weapon = bleeding_ganger(&mut app, PLAYER, LifeState::Alive, start);
    weapon_down(&mut app, by_weapon, start);
    // The injury path: Alive at the boundary; round 1's tick downs them.
    let by_bleed = bleeding_ganger(&mut app, PLAYER, LifeState::Alive, start);
    app.world_mut()
        .entity_mut(by_bleed)
        .insert((Hp::new(ACCRUAL), BleedAfflicted::new(ACCRUAL)));

    end_turn(&mut app); // round 1
    assert_eq!(
        wounds_of(&app, by_weapon),
        start - rate,
        "round 1: the weapon-downed ganger ENTERED the tick Downed — it drains one rate",
    );
    assert_eq!(
        life_of(&app, by_bleed),
        LifeState::Downed,
        "round 1's tick downs the injury-bleed ganger",
    );
    assert_eq!(
        wounds_of(&app, by_bleed),
        start,
        "round 1: the injury-downed ganger was downed BY the tick — it drains nothing",
    );

    end_turn(&mut app); // round 2
    assert_eq!(
        wounds_of(&app, by_weapon),
        start - rate * 2,
        "round 2: the weapon-downed ganger drains its second rate",
    );
    assert_eq!(
        wounds_of(&app, by_bleed),
        start - rate,
        "round 2: the injury-downed ganger's FIRST entered-Downed tick drains exactly \
         one rate — the same first-tick behavior as the weapon path (GTW-641 A2)",
    );
}
