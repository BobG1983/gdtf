use super::support::{
    BleedingOut, PLAYER, bleed_rate, bleeding_ganger, end_turn, life_of, live_app, wounds_of,
};
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

const ACCRUAL: u16 = 4;

#[test]
fn a_ganger_downed_by_the_injury_bleed_keeps_its_wounds_that_round() {
    let rate = bleed_rate();
    let start = rate.saturating_add(10);

    let mut app = live_app();
    let ganger = bleeding_ganger(&mut app, PLAYER, LifeState::Alive, start);
    app.world_mut()
        .entity_mut(ganger)
        .insert((Hp::new(ACCRUAL), BleedAfflicted::new(ACCRUAL)));

    end_turn(&mut app); 

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

#[test]
fn a_ganger_downed_by_the_injury_bleed_cannot_die_the_same_tick() {
    let rate = bleed_rate();
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

fn weapon_down(app: &mut bevy::prelude::App, ganger: bevy::prelude::Entity, start_wounds: u8) {
    let mut hp = Hp::new(ACCRUAL);
    let mut wounds = Wounds::new(start_wounds);
    let mut life = LifeState::Alive;
    let mut inflicted = InflictedWounds::default();
    let tuning = CombatTuning::default();
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

#[test]
fn weapon_and_injury_downs_share_the_first_downed_tick_wound_behavior() {
    let rate = bleed_rate();
    let start = rate.saturating_mul(2).saturating_add(10);

    let mut app = live_app();
    let by_weapon = bleeding_ganger(&mut app, PLAYER, LifeState::Alive, start);
    weapon_down(&mut app, by_weapon, start);
    let by_bleed = bleeding_ganger(&mut app, PLAYER, LifeState::Alive, start);
    app.world_mut()
        .entity_mut(by_bleed)
        .insert((Hp::new(ACCRUAL), BleedAfflicted::new(ACCRUAL)));

    app.update();
    assert!(
        app.world().get::<BleedingOut>(by_weapon).is_some(),
        "the live mark_downed_bleeding must reify the apply_hit down as the BleedingOut \
         condition (no hand-insert)",
    );

    end_turn(&mut app); 
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

    end_turn(&mut app); 
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
