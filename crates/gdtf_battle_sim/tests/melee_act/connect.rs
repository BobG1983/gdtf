//! C6(a) + C6(e) — the landed strike: damage, wear, and the `MeleeStruck` / `ArmorBroken`
//! emissions, with their connect-only log machinery.

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

/// Every `MeleeStruck` observed across the run — the GTW-572 number-bearing melee fact the
/// combat log's melee-damage line reads (attacker + target + applied HP loss).
#[derive(Resource, Default)]
pub(crate) struct StruckLog {
    /// One entry per `MeleeStruck` emitted.
    facts: Vec<gdtf_battle_sim::acts::MeleeStruck>,
}

/// Drain `MeleeStruck` into the recorder.
pub(crate) fn record_struck(
    mut struck: bevy::prelude::MessageReader<gdtf_battle_sim::acts::MeleeStruck>,
    mut log: bevy::prelude::ResMut<StruckLog>,
) {
    for fact in struck.read() {
        log.facts.push(*fact);
    }
}

/// The recorded `MeleeStruck` facts across the run.
fn struck_facts(app: &App) -> Vec<gdtf_battle_sim::acts::MeleeStruck> {
    app.world()
        .get_resource::<StruckLog>()
        .map_or_else(Vec::new, |log| log.facts.clone())
}

/// Every `ArmorBroken` observed across the run — the GTW-572 protecting→broken fact the
/// armor-broken pop + combat-log line drain. Recorded here so the connect test pins the
/// REAL melee emission path (the verb's surfaced `MeleeStrike.wear` → the dispatch
/// bridge → the buffered message), not a hand-written buffer write.
#[derive(Resource, Default)]
pub(crate) struct BrokenLog {
    /// One entry per `ArmorBroken` emitted.
    facts: Vec<ArmorBroken>,
}

/// Drain `ArmorBroken` into the recorder.
pub(crate) fn record_broken(
    mut broken: bevy::prelude::MessageReader<ArmorBroken>,
    mut log: bevy::prelude::ResMut<BrokenLog>,
) {
    for fact in broken.read() {
        log.facts.push(*fact);
    }
}

/// The recorded `ArmorBroken` facts across the run.
fn broken_facts(app: &App) -> Vec<ArmorBroken> {
    app.world()
        .get_resource::<BrokenLog>()
        .map_or_else(Vec::new, |log| log.facts.clone())
}

/// Reduce every worn piece on `ganger` to a NEAR-BROKEN integrity (1), returning how
/// many pieces were reduced (the caller asserts the fixture actually wears armor).
///
/// The §5 formula never reads integrity magnitude (only the `> 0` protects gate), so
/// this leaves the connect test's damage / wound / TU outcomes unchanged — it only
/// guarantees the connecting strike's positive wear (`min(protection, damage) ≥ 2` for
/// the test armor) CROSSES the struck piece protecting→broken.
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

// === C6(a) + C6(e) — a forced connect applies damage end-to-end: HP down + Wound + TU spent +
// MeleeResolved emitted (the FX signal proves the act resolves end-to-end on the real path). ===

#[test]
fn connect_applies_damage_and_emits_resolved() {
    let mut app = battle_app();
    with_melee_log(&mut app);

    // The player attacker faces East at (5,5); the defenceless enemy stands 8-adjacent at (6,5)
    // (directly east, clear LOS, no cover between them).
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

    // GTW-572: reduce the target's worn test armor to NEAR-BROKEN (integrity 1) so the
    // forced connect's positive wear crosses it protecting→broken — pinning the melee
    // ArmorBroken emission on the real path (verb wear verdict → dispatch bridge →
    // buffered fact). Integrity magnitude never feeds §5, so every other assert below
    // is untouched.
    let reduced = wear_pieces_near_broken(&mut app, target);
    assert!(
        reduced > 0,
        "fixture precondition: the target wears the test armor (pieces to reduce)",
    );

    // Drive the strike THROUGH the buffered MeleeRequested (the message the input layer writes).
    app.world_mut()
        .write_message(MeleeRequested::new(attacker, target));
    step(&mut app, 3);

    // C6(a): the target's HP went DOWN (a connecting hit applies the multiplied §5/§6 damage).
    let Some(target_hp_after) = hp_of(&app, target) else {
        unreachable!("the target persists");
    };
    assert!(
        target_hp_after < target_hp_before,
        "C6(a): a connecting melee hit takes the target's HP DOWN ({target_hp_after} < \
         {target_hp_before})",
    );

    // C6(a): a Wound was recorded (the §6 severity tier spent from the Wounds pool). The forced
    // connect at mult_max + the defenceless target makes a non-graze wound the expected outcome;
    // structurally, Wounds did not RISE and the pool moved (a wound was registered).
    let Some(target_wounds_after) = wounds_of(&app, target) else {
        unreachable!("the target persists");
    };
    assert!(
        target_wounds_after <= target_wounds_before,
        "C6(a): the connecting hit never RAISES the target's Wounds",
    );
    // The fat-HP defenceless target survives but takes a real wound: assert a Wound was spent
    // OR (if the multiplied blow was lethal) the target is Dead — either way the §6 step ran.
    let target_downed_or_dead = matches!(
        life_of(&app, target),
        Some(LifeState::Downed | LifeState::Dead)
    );
    assert!(
        target_wounds_after < target_wounds_before || target_downed_or_dead,
        "C6(a): the §6 wound step ran — a Wound was spent (or the blow was lethal): \
         {target_wounds_before} → {target_wounds_after}",
    );

    // C6(a): the attacker's TU was spent (the fight-mode flat TU charge).
    let Some(attacker_tu_after) = tu_of(&app, attacker) else {
        unreachable!("the attacker persists");
    };
    assert!(
        attacker_tu_after < attacker_tu_before,
        "C6(a): the attacker's TU is spent by the strike ({attacker_tu_after} < \
         {attacker_tu_before})",
    );

    // C6(a) + C6(e): a MeleeResolved was emitted (the strike-landed FX signal) — the act
    // resolved end-to-end on the real runtime path. PIN-DISCRIMINATING (fails if unwired).
    assert!(
        melee_hits(&app) >= 1,
        "C6(a)/C6(e): a connecting strike emits MeleeResolved (the FX signal) — the live melee \
         act is wired end-to-end",
    );

    // GTW-572: the connecting strike ALSO emits the number-bearing MeleeStruck fact — both
    // combatants named, carrying the strike's RESOLVED HP damage (the sim emits the FACT the
    // combat log's melee-damage line phrases). The resolved number is at LEAST the observed
    // pool delta (apply_hit saturates the pool at 0, so an overkill blow drains fewer HP than
    // it resolved — the fact carries the blow, the pool carries the floor).
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

    // GTW-572: the connecting strike on the NEAR-BROKEN worn piece crossed it
    // protecting→broken, and the dispatch bridge surfaced the verb's wear verdict as
    // EXACTLY ONE buffered ArmorBroken naming the struck ganger — the melee emission
    // pin (reverting the resolve.rs bridge or the MeleeStrike.wear surfacing fails it).
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
