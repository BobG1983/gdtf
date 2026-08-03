use bevy::transform::components::Transform;
use gdtf_battle_presenter::{
    FctValence, FloatingCombatText, cell_to_world, severity_color, valence_color,
};
use gdtf_battle_sim::{
    armor::BodyPart,
    prelude::{BattleInProgress, Cell, CellLevel, Level, LifeState, Position, SimPos},
    resolve_and_apply::HitReport,
    resolve_coarse::ShotKind,
    sample_cone::ShotDir,
    severity::Severity,
    shot_fired::ShotFired,
    weapon::DamageType,
};

use super::{harness::*, probes::*};

#[test]
fn shot_fired_with_a_lethal_hit_spawns_the_classified_fct_pops() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);
    app.world_mut().insert_resource(BattleInProgress);

    let cell = Cell::new(6, 4);
    let level = Level::new(0);
    let struck = app
        .world_mut()
        .spawn(Position::new(CellLevel::new(cell, level)))
        .id();

    let report = ganger_hit_report(
        struck,
        BodyPart::Torso,
        9,
        6,
        Severity::Critical,
        LifeState::Dead,
    );
    let shot = ShotFired {
        shooter:      app.world_mut().spawn_empty().id(),
        muzzle:       SimPos::new(1.0, 1.0, 0.0),
        trajectory:   ShotDir::from_direction(bevy::math::Vec3::new(1.0, 0.0, 0.0)),
        impact_cell:  cell,
        impact_level: level,
        kind:         ShotKind::Ganger(struck),
        damage:       DamageType::Kinetic,
        report:       Some(report),
    };
    play(&mut app, shot);
    fire_with_zero_delta(&mut app);
    step_app(&mut app, std::time::Duration::from_millis(50), 8);

    let pops = fct_pops(&mut app);
    assert!(
        has_fct_pop(&pops, "-9", valence_color(FctValence::Damage)),
        "the 9-HP hit must pop a RED \"-9\", got {pops:?}",
    );
    assert!(
        has_fct_pop(&pops, "Torso Critical", severity_color(Severity::Critical)),
        "a Critical torso wound must pop \"Torso Critical\" in the Critical amber, got {pops:?}",
    );
    assert!(
        has_fct_pop(&pops, "Armor pierced", valence_color(FctValence::Neutral)),
        "a penetrating hit must pop a GREY \"Armor pierced\", got {pops:?}",
    );
    assert!(
        has_fct_pop(&pops, "DEAD", valence_color(FctValence::Lethal)),
        "a Dead outcome must pop a lethal-RED \"DEAD\", got {pops:?}",
    );

    let anchor = cell_to_world(cell, level);
    let mut q = app.world_mut().query::<(&FloatingCombatText, &Transform)>();
    let any_at_cell = q
        .iter(app.world())
        .any(|(_, transform)| (transform.translation.x - anchor.x).abs() < 0.001);
    assert!(
        any_at_cell,
        "the FCT pops must anchor at the hit ganger's cell x ({})",
        anchor.x,
    );
}

#[test]
fn shot_fired_clean_miss_pops_nothing() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);
    app.world_mut().insert_resource(BattleInProgress);

    let cell = Cell::new(2, 9);
    let level = Level::new(0);
    let shot = ShotFired {
        shooter:      app.world_mut().spawn_empty().id(),
        muzzle:       SimPos::new(1.0, 1.0, 0.0),
        trajectory:   ShotDir::from_direction(bevy::math::Vec3::new(0.0, 1.0, 0.0)),
        impact_cell:  cell,
        impact_level: level,
        kind:         ShotKind::Miss,
        damage:       DamageType::Kinetic,
        report:       Some(HitReport::no_effect(ShotKind::Miss)),
    };
    play(&mut app, shot);
    fire_with_zero_delta(&mut app);
    step_app(&mut app, std::time::Duration::from_millis(50), 8);

    let pops = fct_pops(&mut app);
    assert!(
        pops.is_empty(),
        "a clean miss must spawn no FCT pop, got {pops:?}"
    );
}
