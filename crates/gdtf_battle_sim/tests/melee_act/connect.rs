use bevy::{
    app::App,
    prelude::{Entity, Resource},
};
use gdtf_battle_sim::{
    acts::MeleeRequested,
    armor::{ArmorIntegrity, Wears},
    armor_wear::ArmorBroken,
    ganger::Direction,
    prelude::LifeState,
    test_support::SituationBuilder,
};

use super::harness::*;

#[derive(Resource, Default)]
pub(crate) struct StruckLog {
        facts: Vec<gdtf_battle_sim::acts::MeleeStruck>,
}

pub(crate) fn record_struck(
    mut struck: bevy::prelude::MessageReader<gdtf_battle_sim::acts::MeleeStruck>,
    mut log: bevy::prelude::ResMut<StruckLog>,
) {
    for fact in struck.read() {
        log.facts.push(*fact);
    }
}

fn struck_facts(app: &App) -> Vec<gdtf_battle_sim::acts::MeleeStruck> {
    app.world()
        .get_resource::<StruckLog>()
        .map_or_else(Vec::new, |log| log.facts.clone())
}

#[derive(Resource, Default)]
pub(crate) struct BrokenLog {
        facts: Vec<ArmorBroken>,
}

pub(crate) fn record_broken(
    mut broken: bevy::prelude::MessageReader<ArmorBroken>,
    mut log: bevy::prelude::ResMut<BrokenLog>,
) {
    for fact in broken.read() {
        log.facts.push(*fact);
    }
}

fn broken_facts(app: &App) -> Vec<ArmorBroken> {
    app.world()
        .get_resource::<BrokenLog>()
        .map_or_else(Vec::new, |log| log.facts.clone())
}

fn wear_pieces_near_broken(app: &mut App, ganger: Entity) -> usize {
    let pieces: Vec<Entity> = app
        .world()
        .get::<Wears>(ganger)
        .map(|wears| wears.pieces().collect())
        .unwrap_or_default();
    for &piece in &pieces {
        if let Some(mut integrity) = app.world_mut().get_mut::<ArmorIntegrity>(piece) {
            *integrity = ArmorIntegrity::new(1);
        }
    }
    pieces.len()
}


#[test]
fn connect_applies_damage_and_emits_resolved() {
    let mut app = battle_app();
    with_melee_log(&mut app);

    let situation = SituationBuilder::new()
        .with_gangers([
            strong_attacker(ground(5, 5), PLAYER, Direction::East),
            defenceless_target(ground(6, 5), ENEMY),
        ])
        .build_with_gangs();
    drive_setup(&mut app, situation);

    let (Some(attacker), Some(target)) = (ganger_of(&mut app, PLAYER), ganger_of(&mut app, ENEMY))
    else {
        unreachable!("setup spawns one player attacker and one enemy target");
    };
    let (Some(attacker_tu_before), Some(target_hp_before), Some(target_wounds_before)) = (
        tu_of(&app, attacker),
        hp_of(&app, target),
        wounds_of(&app, target),
    ) else {
        unreachable!("both gangers carry Tu / Hp / Wounds pools");
    };

    let reduced = wear_pieces_near_broken(&mut app, target);
    assert!(
        reduced > 0,
        "fixture precondition: the target wears the test armor (pieces to reduce)",
    );

    app.world_mut()
        .write_message(MeleeRequested::new(attacker, target));
    step(&mut app, 3);

    let Some(target_hp_after) = hp_of(&app, target) else {
        unreachable!("the target persists");
    };
    assert!(
        target_hp_after < target_hp_before,
        "C6(a): a connecting melee hit takes the target's HP DOWN ({target_hp_after} < \
         {target_hp_before})",
    );

    let Some(target_wounds_after) = wounds_of(&app, target) else {
        unreachable!("the target persists");
    };
    assert!(
        target_wounds_after <= target_wounds_before,
        "C6(a): the connecting hit never RAISES the target's Wounds",
    );
    let target_downed_or_dead = matches!(
        life_of(&app, target),
        Some(LifeState::Downed | LifeState::Dead)
    );
    assert!(
        target_wounds_after < target_wounds_before || target_downed_or_dead,
        "C6(a): the §6 wound step ran — a Wound was spent (or the blow was lethal): \
         {target_wounds_before} → {target_wounds_after}",
    );

    let Some(attacker_tu_after) = tu_of(&app, attacker) else {
        unreachable!("the attacker persists");
    };
    assert!(
        attacker_tu_after < attacker_tu_before,
        "C6(a): the attacker's TU is spent by the strike ({attacker_tu_after} < \
         {attacker_tu_before})",
    );

    assert!(
        melee_hits(&app) >= 1,
        "C6(a)/C6(e): a connecting strike emits MeleeResolved (the FX signal) — the live melee \
         act is wired end-to-end",
    );

    let facts = struck_facts(&app);
    let observed_loss = i32::from(target_hp_before) - i32::from(target_hp_after);
    assert!(
        facts.iter().any(|fact| fact.attacker == attacker
            && fact.target == target
            && *fact.hp_damage >= observed_loss
            && *fact.hp_damage > 0),
        "GTW-572: a connecting strike emits one MeleeStruck {{ attacker, target, hp_damage }} \
         whose resolved amount is positive and at least the observed HP delta \
         ({observed_loss}), got {facts:?}",
    );

    let breaks = broken_facts(&app);
    assert_eq!(
        breaks.len(),
        1,
        "GTW-572: a connecting strike that crosses a near-broken worn piece emits exactly \
         one ArmorBroken, got {breaks:?}",
    );
    assert!(
        breaks.first().is_some_and(|broke| broke.ganger == target),
        "GTW-572: the ArmorBroken names the struck melee target, got {breaks:?}",
    );
}
