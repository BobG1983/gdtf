use bevy::app::App;
use gdtf_battle_presenter::FxTuning;
use gdtf_battle_sim::{
    armor::BodyPart,
    prelude::{BattleInProgress, Cell, CellLevel, Level, LifeState, Position, SimPos},
    resolve_coarse::ShotKind,
    sample_cone::ShotDir,
    severity::Severity,
    shot_fired::ShotFired,
    weapon::DamageType,
};

use super::{harness::*, probes::*};

const TARGET_CELL: Cell = Cell::new(5, 5);
const TARGET_LEVEL: Level = Level::new(0);

fn connecting_round(app: &mut App, struck: bevy::ecs::entity::Entity) -> ShotFired {
    let report = ganger_hit_report(
        struck,
        BodyPart::Torso,
        4,
        6,
        Severity::None,
        LifeState::Alive,
    );
    ShotFired {
        shooter:      app.world_mut().spawn_empty().id(),
        muzzle:       SimPos::new(4.0, 5.0, 0.0),
        trajectory:   ShotDir::from_direction(bevy::math::Vec3::new(1.0, 0.0, 0.0)),
        impact_cell:  TARGET_CELL,
        impact_level: TARGET_LEVEL,
        kind:         ShotKind::Ganger(struck),
        damage:       DamageType::Kinetic,
        report:       Some(report),
    }
}

#[test]
fn a_multi_round_volley_pops_its_fct_staggered_per_impact() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);
    app.world_mut().insert_resource(BattleInProgress);

    let tuning = app.world().get_resource::<FxTuning>().copied();
    assert!(
        tuning.is_some(),
        "FxTuning must be resident after settle_resources",
    );
    let Some(tuning) = tuning else { return };
    let ttl = std::time::Duration::from_secs_f32(*tuning.fct_ttl_seconds);

    let struck_a = app
        .world_mut()
        .spawn(Position::new(CellLevel::new(TARGET_CELL, TARGET_LEVEL)))
        .id();
    let struck_b = app
        .world_mut()
        .spawn(Position::new(CellLevel::new(TARGET_CELL, TARGET_LEVEL)))
        .id();

    let round_a = connecting_round(&mut app, struck_a);
    play(&mut app, round_a);

    fire_with_zero_delta(&mut app);
    assert_eq!(
        fct_pop_count(&mut app),
        0,
        "at the drain frame NO pop may exist — the whole point of the fix is that the numbers \
         do not all appear at once on the round's drain frame",
    );

    let short_step = std::time::Duration::from_millis(30);
    step_app(&mut app, short_step, 4);
    let after_first = fct_pop_count(&mut app);
    assert!(
        after_first >= 1,
        "after the first bolt's flight its FCT pop(s) must be up (got {after_first})",
    );

    let round_b = connecting_round(&mut app, struck_b);
    play(&mut app, round_b);
    fire_with_zero_delta(&mut app);
    step_app(&mut app, short_step, 4);
    let after_second = fct_pop_count(&mut app);
    assert!(
        after_second > after_first,
        "after the second round's staggered impact MORE pops must be live than after the first \
         ({after_second} must exceed {after_first}) — the second shot's numbers appeared later",
    );

    assert!(
        ttl >= std::time::Duration::from_millis(500),
        "the tuned FCT lifetime ({ttl:?}) is expected to be at least 0.5s (slice-1 fix)",
    );
    step_app(&mut app, std::time::Duration::from_millis(50), 1);
    assert!(
        fct_pop_count(&mut app) >= after_first,
        "a freshly-impacted pop must persist for its tuned lifetime, not vanish on the next frame",
    );
}

#[test]
fn a_multi_round_volley_emits_shot_impact_resolved_staggered_per_impact() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);
    app.world_mut().insert_resource(BattleInProgress);

    let struck_a = app
        .world_mut()
        .spawn(Position::new(CellLevel::new(TARGET_CELL, TARGET_LEVEL)))
        .id();
    let struck_b = app
        .world_mut()
        .spawn(Position::new(CellLevel::new(TARGET_CELL, TARGET_LEVEL)))
        .id();

    let round_a = connecting_round(&mut app, struck_a);
    play(&mut app, round_a);

    fire_with_zero_delta(&mut app);
    assert_eq!(
        drain_impacts(&mut app).len(),
        0,
        "at the drain frame NO ShotImpactResolved may fire — the whole point of the fix is that \
         the shot-outcome lines do not all appear at once on the round's drain frame",
    );

    let short_step = std::time::Duration::from_millis(30);
    let after_first = step_counting_impacts(&mut app, short_step, 4);
    assert!(
        after_first >= 1,
        "after the first bolt's flight its ShotImpactResolved must have fired (got {after_first})",
    );

    let round_b = connecting_round(&mut app, struck_b);
    play(&mut app, round_b);
    fire_with_zero_delta(&mut app);
    let after_second = after_first + step_counting_impacts(&mut app, short_step, 4);
    assert!(
        after_second > after_first,
        "after the second round's staggered impact MORE ShotImpactResolved must have fired in \
         total ({after_second} must exceed {after_first}) — the second shot's outcome resolved later",
    );

    assert_eq!(
        after_second, 2,
        "a two-round volley must emit exactly two ShotImpactResolved (one per shot), staggered, \
         got {after_second}",
    );
}
