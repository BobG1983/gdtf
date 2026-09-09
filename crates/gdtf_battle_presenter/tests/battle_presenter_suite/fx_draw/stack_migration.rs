use std::time::Duration;

use bevy::math::Vec3;
use gdtf_battle_presenter::cell_to_world;
use gdtf_battle_sim::{
    armor::BodyPart,
    falls::{FallOccurred, StoreysFallen},
    prelude::{BattleInProgress, Cell, CellLevel, Level, LifeState, Position, SimPos},
    resolve_coarse::ShotKind,
    sample_cone::ShotDir,
    severity::Severity,
    shot_fired::ShotFired,
    suppression::SuppressionApplied,
    weapon::DamageType,
};

use super::{harness::*, probes::*};

const CELL: Cell = Cell::new(7, 3);
const LEVEL: Level = Level::new(0);

fn connecting_shot(
    app: &mut bevy::app::App,
    struck: bevy::ecs::entity::Entity,
    hp: i32,
    pen: i32,
    severity: Severity,
    life: LifeState,
) -> ShotFired {
    let report = ganger_hit_report(struck, BodyPart::Torso, hp, pen, severity, life);
    ShotFired {
        shooter:      app.world_mut().spawn_empty().id(),
        muzzle:       SimPos::new((CELL.x - 1) as f32, CELL.y as f32, 0.0),
        trajectory:   ShotDir::from_direction(Vec3::new(1.0, 0.0, 0.0)),
        impact_cell:  CELL,
        impact_level: LEVEL,
        kind:         ShotKind::Ganger(struck),
        damage:       DamageType::Kinetic,
        report:       Some(report),
    }
}

#[test]
fn a_shot_pop_stacks_above_a_live_consequence_pop_on_the_same_cell() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);
    app.world_mut().insert_resource(BattleInProgress);

    let at = CellLevel::new(CELL, LEVEL);
    let struck = wounded_ganger(&mut app, CELL, LEVEL, 2);

    play(&mut app, SuppressionApplied::new(struck, at));
    app.update();
    assert!(
        fct_pop_count(&mut app) >= 1,
        "the suppression consequence pop must be live before the shot is fired",
    );

    let shot = connecting_shot(&mut app, struck, 7, 0, Severity::None, LifeState::Alive);
    play(&mut app, shot);
    fire_with_zero_delta(&mut app);

    let pops = step_until_pop(&mut app, "-7", Duration::from_millis(30));

    let base_y = cell_to_world(CELL, LEVEL).y;
    let shot_y = pop_y_for(&pops, "-7");
    assert!(
        shot_y.is_some(),
        "the \"-7\" shot-damage pop must be present in the impact-frame snapshot: {pops:?}",
    );
    let Some(shot_y) = shot_y else { return };
    assert!(
        pops.iter().any(|(t, _)| t == "SUPPRESSED"),
        "the consequence pop must still be live on the cell when the shot lands: {pops:?}",
    );
    assert!(
        shot_y < base_y - 1.0,
        "the shot-damage pop must be seeded ABOVE the live consequence pop (a distinct slot >= 1, so its spawn y sits below the slot-0 base y {base_y}), got {shot_y}: {pops:?}",
    );
}

#[test]
fn a_single_shots_multi_pop_fan_out_ascends_seeded_above_a_live_pop() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);
    app.world_mut().insert_resource(BattleInProgress);

    let at = CellLevel::new(CELL, LEVEL);
    let struck = wounded_ganger(&mut app, CELL, LEVEL, 2);

    play(&mut app, SuppressionApplied::new(struck, at));
    app.update();

    let shot = connecting_shot(&mut app, struck, 9, 6, Severity::Critical, LifeState::Dead);
    play(&mut app, shot);
    fire_with_zero_delta(&mut app);

    let pops = step_until_pop(&mut app, "-9", Duration::from_millis(30));

    let base_y = cell_to_world(CELL, LEVEL).y;
    let shot_texts = ["-9", "Torso Critical", "Armor pierced", "DEAD"];
    let mut ys = Vec::new();
    for text in shot_texts {
        let maybe_y = pop_y_for(&pops, text);
        assert!(
            maybe_y.is_some(),
            "the shot must pop \"{text}\" at its impact: {pops:?}",
        );
        let Some(y) = maybe_y else { continue };
        assert!(
            y < base_y - 1.0,
            "every one of the shot's pops must be seeded ABOVE the live consequence pop (below the \
             slot-0 base y {base_y}); \"{text}\" landed at {y}, colliding with slot 0: {pops:?}",
        );
        ys.push(y);
    }
    for i in 0..ys.len() {
        for j in (i + 1)..ys.len() {
            assert!(
                (ys[i] - ys[j]).abs() > 1.0,
                "a single shot's pops must fan out to DISTINCT stacking slots (distinct ys); \
                 \"{}\"={} and \"{}\"={} collided: {pops:?}",
                shot_texts[i],
                ys[i],
                shot_texts[j],
                ys[j],
            );
        }
    }
}

#[test]
fn two_falls_on_one_cell_across_consecutive_frames_stack() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);
    app.world_mut().insert_resource(BattleInProgress);
    app.world_mut()
        .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            Duration::ZERO,
        ));

    let from_level = Level::new(2);
    let faller_a = app
        .world_mut()
        .spawn(Position::new(CellLevel::new(CELL, LEVEL)))
        .id();
    let faller_b = app
        .world_mut()
        .spawn(Position::new(CellLevel::new(CELL, LEVEL)))
        .id();

    play(
        &mut app,
        FallOccurred::new(faller_a, from_level, LEVEL, StoreysFallen::new(2)),
    );
    app.update();
    let after_first: Vec<_> = fct_pops_with_y(&mut app)
        .into_iter()
        .filter(|(t, _)| t == "Fell")
        .collect();
    assert_eq!(
        after_first.len(),
        1,
        "exactly one \"Fell\" pop must be live after the first fall, got {after_first:?}",
    );

    play(
        &mut app,
        FallOccurred::new(faller_b, from_level, LEVEL, StoreysFallen::new(1)),
    );
    app.update();
    app.world_mut()
        .insert_resource(bevy::time::TimeUpdateStrategy::Automatic);

    let fell_ys: Vec<f32> = fct_pops_with_y(&mut app)
        .into_iter()
        .filter(|(t, _)| t == "Fell")
        .map(|(_, y)| y)
        .collect();
    assert_eq!(
        fell_ys.len(),
        2,
        "both falls' \"Fell\" pops must be live after the second fall, got {fell_ys:?}",
    );
    assert!(
        (fell_ys[0] - fell_ys[1]).abs() > 1.0,
        "two falls co-occurring on one cell must take DISTINCT stack slots (distinct ys), got {fell_ys:?}",
    );
}
