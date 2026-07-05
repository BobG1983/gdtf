//! QA(7) / QA(8c) / QA(8d) — the shove-tagged RANGED hook: a connecting tagged shot knocks
//! back; an untagged connect and a wall-stopped miss do not.

use gdtf_battle_sim::{
    Cell,
    ganger::Direction,
    test_support::SituationBuilder,
    weapon::{FireModeSpec, ModeConeMult, ModeKind, ModeShots, ModeTuPercent},
};

use super::harness::*;

// ── QA(7) — a shove-tagged RANGED weapon knocks back on a connecting shot ──────

#[test]
fn shove_tagged_ranged_connect_knocks_target_back() {
    // A shove-tagged ranged weapon; melee untagged (irrelevant — this is a shot).
    let mut app = battle_app(false, true);
    // Player shooter faces East at (5,5); enemy point-blank East at (6,5) (a near-certain
    // connect with the tight-cone / high-accuracy test gun); the knock-back cell (7,5) is open.
    let situation = SituationBuilder::new()
        .with_gangers([
            strong_attacker(ground(5, 5), PLAYER, Direction::East),
            defenceless_target(ground(6, 5), ENEMY),
        ])
        .build_with_gangs();
    drive_setup(&mut app, situation);
    let (Some(shooter), Some(target)) = (ganger_of(&mut app, PLAYER), ganger_of(&mut app, ENEMY))
    else {
        unreachable!("setup spawns one player + one enemy");
    };
    assert_eq!(
        pos_of(&app, target),
        Some(ground(6, 5)),
        "target starts at (6,5)"
    );

    // Fire the ranged weapon at the enemy's cell (a single-shot, near-guaranteed point-blank
    // connect with the tight cone). dispatch_fire's connect hook writes the weapon-tag shove.
    app.world_mut()
        .write_message(gdtf_battle_sim::acts::FireRequested::new(
            shooter,
            single_shot_mode(),
            Cell::new(6, 5),
            level0(),
        ));
    step(&mut app, 4);

    assert_eq!(
        pos_of(&app, target),
        Some(ground(7, 5)),
        "a `shove`-tagged gun knocks the target back one cell on a connecting shot"
    );
}

/// A single-shot fire-mode spec matching the ranged test weapon's authored mode (the message
/// carries an OWNED `FireModeSpec` — the input seam's shape).
const fn single_shot_mode() -> FireModeSpec {
    FireModeSpec::new(
        ModeKind::Single,
        ModeConeMult::new(1.0),
        ModeTuPercent::new(0.2),
        ModeShots::new(1),
    )
}

// ── QA(8c) — a NON-shove RANGED weapon never shoves on a connecting shot ────────

#[test]
fn non_shove_ranged_connect_does_not_knock_back() {
    // Both weapons UNtagged — the SAME point-blank connect geometry as QA(7), but the gun is
    // NOT `shove`-tagged, so the connecting round must NOT move the target. This is the
    // fire-site negative for `dispatch_fire`'s `weapon_shoves` tag-read: dropping that guard
    // (an unconditional shove on ANY connecting round) would move the target here and fail.
    let mut app = battle_app(false, false);
    let situation = SituationBuilder::new()
        .with_gangers([
            strong_attacker(ground(5, 5), PLAYER, Direction::East),
            defenceless_target(ground(6, 5), ENEMY),
        ])
        .build_with_gangs();
    drive_setup(&mut app, situation);
    let (Some(shooter), Some(target)) = (ganger_of(&mut app, PLAYER), ganger_of(&mut app, ENEMY))
    else {
        unreachable!("setup spawns one player + one enemy");
    };
    assert_eq!(
        pos_of(&app, target),
        Some(ground(6, 5)),
        "target starts at (6,5)"
    );

    // Fire the UNtagged ranged weapon at the enemy's cell — a near-guaranteed point-blank
    // connect (the same tight-cone shot as QA(7)); the connect happens, only the tag differs.
    app.world_mut()
        .write_message(gdtf_battle_sim::acts::FireRequested::new(
            shooter,
            single_shot_mode(),
            Cell::new(6, 5),
            level0(),
        ));
    step(&mut app, 4);

    assert_eq!(
        pos_of(&app, target),
        Some(ground(6, 5)),
        "a NON-`shove` gun never knocks the target back — even on a connecting shot"
    );
}

// ── QA(8d) — a MISSED shot never shoves (even with a shove-tagged gun) ──────────

#[test]
fn shove_tagged_ranged_miss_does_not_knock_back() {
    // A `shove`-tagged ranged weapon, aimed AT the target's cell — but a WALL is interposed
    // between shooter and target, so the round deterministically STOPS on the wall (a
    // `ShotKind::Cover`/`Wall` outcome) and never reaches the ganger → no `ShotKind::Ganger`
    // round → `struck_ganger` is None → NO shove. This is the fire-site negative for
    // `dispatch_fire`'s connect-read (`struck_ganger`): the ganger IS the aimed occupant, so a
    // regression that shoved the AIMED occupant (or shoved on any tagged fire regardless of a
    // connect) would move the target here and FAIL — only the connect gate keeps it still.
    let mut app = battle_app(false, true);
    // Shooter faces East at (5,5); a wall fills (6,5); the target stands beyond it at (7,5).
    // The East shot marches into the wall at (6,5) and stops — the target is aimed-at but
    // shielded (a deterministic geometric miss, no RNG-dependent cone dodge).
    let situation = SituationBuilder::new()
        .with_gangers([
            strong_attacker(ground(5, 5), PLAYER, Direction::East),
            defenceless_target(ground(7, 5), ENEMY),
        ])
        .wall_at(ground(6, 5))
        .build_with_gangs();
    drive_setup(&mut app, situation);
    let (Some(shooter), Some(target)) = (ganger_of(&mut app, PLAYER), ganger_of(&mut app, ENEMY))
    else {
        unreachable!("setup spawns one player + one enemy");
    };
    assert_eq!(
        pos_of(&app, target),
        Some(ground(7, 5)),
        "target starts at (7,5), behind the wall at (6,5)"
    );

    // Fire straight East AT the target's cell (7,5) — in-arc (no turn); the round is stopped by
    // the wall at (6,5) before reaching the ganger, so no round connects with a ganger.
    app.world_mut()
        .write_message(gdtf_battle_sim::acts::FireRequested::new(
            shooter,
            single_shot_mode(),
            Cell::new(7, 5),
            level0(),
        ));
    step(&mut app, 4);

    assert_eq!(
        pos_of(&app, target),
        Some(ground(7, 5)),
        "a MISSED shot never shoves — even with a `shove`-tagged gun (the connect gate held)"
    );
}
