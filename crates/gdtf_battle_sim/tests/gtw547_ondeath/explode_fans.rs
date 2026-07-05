//! The explode-on-death effect across every kill path: a shot kill, a melee kill,
//! and a Blast splash kill each fan the dead ganger's on-death Explode.

use gdtf_battle_sim::{
    LifeState,
    acts::{FireRequested, MeleeRequested},
    ganger::Direction,
    metric::{Cell, Level},
    test_support::{SituationBuilder, single_mode},
    weapon::{
        BlastRadius, FireModeSpec, HitType, ModeConeMult, ModeKind, ModeShots, ModeTuPercent,
    },
};

use super::harness::*;

// === Explode headline: a killed ganger's on-death blast damages an adjacent ganger. ===

#[test]
fn a_killed_ganger_with_explode_on_death_damages_an_adjacent_ganger() {
    let (mut app, seed) = battle_app(0x5547_0A0A, false);

    // Shooter at (5,5) facing East; the FRAIL victim at (8,5) (the shot kills it); a sturdy
    // BYSTANDER at (8,4) — adjacent (north) to the victim, INSIDE the victim's death-blast
    // radius-1 disc. All one faction (the gtw541 clean-baseline recipe).
    let situation = SituationBuilder::new()
        .with_gangers([
            shooter(ground(5, 5), Direction::East),
            frail_target(ground(8, 5)),
            bystander(ground(8, 4)),
        ])
        .build_with_gangs();
    drive_setup(&mut app, seed, situation);

    let (Some(shooter_e), Some(_victim), Some(neighbour)) = (
        ganger_at(&mut app, ground(5, 5)),
        ganger_at(&mut app, ground(8, 5)),
        ganger_at(&mut app, ground(8, 4)),
    ) else {
        unreachable!("setup spawns the shooter + victim + neighbour at distinct cells");
    };
    let neighbour0 = vitals(&app, neighbour);

    // Fire the lethal shot at the victim's cell. The kill emits OnDeathOccurred; the SAME-frame
    // (or next-tick) resolve_on_death fans the victim's Explode at (8,5), striking (8,4).
    app.world_mut().write_message(FireRequested::new(
        shooter_e,
        single_mode(0.2, 1),
        Cell::new(8, 5),
        Level::new(0),
    ));
    step(&mut app, 4);

    // The neighbour — never shot at directly — LOST HP to the victim's on-death blast.
    // PIN-DISCRIMINATING: with the death→effect bridge or resolve_on_death unwired the
    // neighbour is untouched (only the directly-shot victim would change).
    assert!(
        took_damage(neighbour0, vitals(&app, neighbour)),
        "the adjacent ganger took damage from the killed ganger's on-death Explode blast \
         (before {neighbour0:?}, after {:?})",
        vitals(&app, neighbour),
    );
}

// === Melee ganger-kill gate: a melee-killed ganger fans its (ranged) on-death Explode. ===

#[test]
fn a_ganger_killed_in_melee_fans_its_on_death_explode() {
    let (mut app, seed) = battle_app(0x5547_0C0C, false);
    // The lethal melee weapon so the strike KILLS the frail victim (the melee-kill gate).
    app.world_mut().insert_resource(lethal_melee_registry());

    // A faction-1 attacker at (5,5) facing East; the FRAIL victim (faction 0) 8-adjacent at
    // (6,5) — opposing faction + adjacent + clear LOS (the melee gates). A sturdy BYSTANDER
    // (faction 0) at (6,4) — adjacent (north) to the victim, INSIDE the victim's death-blast
    // radius-1 disc (the ranged Explode weapon rides every setup ganger).
    let situation = SituationBuilder::new()
        .with_gangers([
            melee_attacker(ground(5, 5), 1, Direction::East),
            frail_target(ground(6, 5)),
            bystander(ground(6, 4)),
        ])
        .build_with_gangs();
    drive_setup(&mut app, seed, situation);

    let (Some(attacker), Some(victim), Some(neighbour)) = (
        ganger_at(&mut app, ground(5, 5)),
        ganger_at(&mut app, ground(6, 5)),
        ganger_at(&mut app, ground(6, 4)),
    ) else {
        unreachable!("setup spawns the attacker + victim + neighbour at distinct cells");
    };
    let neighbour0 = vitals(&app, neighbour);

    // Strike the victim in melee THROUGH the buffered MeleeRequested (the input-seam message).
    app.world_mut()
        .write_message(MeleeRequested::new(attacker, victim));
    step(&mut app, 4);

    // The victim died to the melee strike.
    assert_eq!(
        life_of(&app, victim),
        LifeState::Dead,
        "the lethal melee strike killed the victim"
    );
    // The neighbour — never struck directly — LOST HP to the melee-killed victim's on-death
    // Explode blast. PIN-DISCRIMINATING: reverting the melee ganger-kill `deaths.write(...)`
    // leaves the neighbour untouched (only the struck victim would change).
    assert!(
        took_damage(neighbour0, vitals(&app, neighbour)),
        "the adjacent ganger took damage from the melee-killed ganger's on-death Explode blast \
         (before {neighbour0:?}, after {:?})",
        vitals(&app, neighbour),
    );
}

// === Fire splash-kill gate: a Blast-splash-killed ganger fans its own on-death Explode. ===

/// A Blast-`HitType` fire spec on the Explode weapon — a `Single`-kind shot whose GTW-541
/// [`HitType::Blast`] template SPLASHES an `AoE`, letting the splash branch (not just the
/// primary round) exercise the death gate.
const fn blast_mode() -> FireModeSpec {
    FireModeSpec::with_hit_type(
        ModeKind::Single,
        ModeConeMult::new(1.0),
        ModeTuPercent::new(0.2),
        ModeShots::new(1),
        HitType::Blast {
            radius: BlastRadius::new(1),
        },
    )
}

#[test]
fn a_ganger_splash_killed_by_fire_fans_its_on_death_explode() {
    let (mut app, seed) = battle_app(0x5547_0E0E, false);

    // Shooter at (5,5) facing East. A sturdy ANCHOR at (8,5) — the aim cell — so the tight,
    // `stable` round impacts THERE (the gtw541 impact-is-aim-cell recipe: a ganger to stop
    // on), detonating the Blast at (8,5). A frail SPLASH-VICTIM at (8,4) — a splash occupant
    // in the radius-1 disc of (8,5), NOT the direct target — the blast splash KILLS it (the
    // splash-kill gate). A sturdy BYSTANDER at (8,3) — north of the splash-victim, inside ITS
    // OWN on-death blast radius but OUTSIDE the fired blast's radius-1 disc of (8,5). All one
    // faction (the friendly-fire clean-baseline recipe; the Blast is faction-blind).
    let situation = SituationBuilder::new()
        .with_gangers([
            shooter(ground(5, 5), Direction::East),
            bystander(ground(8, 5)),
            frail_target(ground(8, 4)),
            bystander(ground(8, 3)),
        ])
        .build_with_gangs();
    drive_setup(&mut app, seed, situation);

    let (Some(shooter_e), Some(splash_victim), Some(neighbour)) = (
        ganger_at(&mut app, ground(5, 5)),
        ganger_at(&mut app, ground(8, 4)),
        ganger_at(&mut app, ground(8, 3)),
    ) else {
        unreachable!("setup spawns the shooter + splash-victim + neighbour at distinct cells");
    };
    let neighbour0 = vitals(&app, neighbour);

    // Fire a Blast aimed at (8,5): the round impacts the anchor there and detonates; the SPLASH
    // catches the frail off-axis victim at (8,4) and KILLS it; its on-death Explode then fans
    // onto the neighbour at (8,3).
    app.world_mut().write_message(FireRequested::new(
        shooter_e,
        blast_mode(),
        Cell::new(8, 5),
        Level::new(0),
    ));
    step(&mut app, 4);

    assert_eq!(
        life_of(&app, splash_victim),
        LifeState::Dead,
        "the Blast splash killed the off-axis victim"
    );
    // The neighbour at (8,3) — OUTSIDE the fired Blast's radius-1 disc of (8,5) — LOST HP only
    // to the splash-killed victim's OWN on-death Explode (whose radius-1 disc of (8,4) reaches
    // (8,3)). PIN-DISCRIMINATING: reverting the fire SPLASH-path `emit_on_death` leaves the
    // neighbour untouched (the splash kill would fan no effect).
    assert!(
        took_damage(neighbour0, vitals(&app, neighbour)),
        "the neighbour took damage from the splash-killed ganger's on-death Explode blast \
         (before {neighbour0:?}, after {:?})",
        vitals(&app, neighbour),
    );
}
