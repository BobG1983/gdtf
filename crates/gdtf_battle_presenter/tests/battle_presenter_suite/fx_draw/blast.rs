use bevy::{app::App, sprite::Sprite, time::TimeUpdateStrategy, transform::components::Transform};
use gdtf_battle_presenter::{ShotProjectile, cell_to_world};
use gdtf_battle_sim::{
    acts::ThrowResolved,
    prelude::{BattleInProgress, Cell, CellLevel, Level},
    weapon::DamageType,
};

use super::harness::*;

fn sprites_at(app: &mut App, at: bevy::math::Vec3) -> usize {
    let mut q = app.world_mut().query_filtered::<&Transform, (
        bevy::prelude::With<Sprite>,
        bevy::prelude::Without<ShotProjectile>,
    )>();
    q.iter(app.world())
        .filter(|transform| transform.translation.distance(at) < 0.01)
        .count()
}

#[test]
fn throw_resolved_draws_the_blast_impact_at_the_landing_cell() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);
    app.world_mut().insert_resource(BattleInProgress);

    let cell = Cell::new(9, 5);
    let level = Level::new(0);
    let landing = CellLevel::new(cell, level);
    let landing_world = cell_to_world(cell, level);

    assert_eq!(
        step_counting_impacts(&mut app, std::time::Duration::from_millis(16), 1),
        0,
        "no impact may resolve before any throw",
    );
    assert_eq!(
        sprites_at(&mut app, landing_world),
        0,
        "no blast glyph may sit at the landing before the throw resolves",
    );

    play(&mut app, ThrowResolved::new(landing, DamageType::Blast));

    let resolved = step_counting_impacts(&mut app, std::time::Duration::from_millis(16), 2);
    assert_eq!(
        resolved, 1,
        "a ThrowResolved must drive EXACTLY one blast impact through the existing pipeline",
    );

    assert!(
        sprites_at(&mut app, landing_world) >= 1,
        "the blast impact glyph must draw at cell_to_world(landing) (the AoE hit FX)",
    );
}

#[test]
fn throw_resolved_blast_impact_carries_no_shot_verdict() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);
    app.world_mut().insert_resource(BattleInProgress);

    let cell = Cell::new(3, 8);
    let level = Level::new(0);
    let landing = CellLevel::new(cell, level);

    play(&mut app, ThrowResolved::new(landing, DamageType::Chem));

    app.world_mut()
        .insert_resource(TimeUpdateStrategy::ManualDuration(
            std::time::Duration::from_millis(16),
        ));
    app.update();
    app.update();

    let impacts = drain_impacts(&mut app);
    assert_eq!(
        impacts.len(),
        1,
        "the blast must emit exactly one ShotImpactResolved as its seed resolves",
    );
    let Some(impact) = impacts.first() else {
        return;
    };
    assert_eq!(
        impact.shooter,
        bevy::ecs::entity::Entity::PLACEHOLDER,
        "a blast has no single shooter verdict — its seed carries the PLACEHOLDER shooter",
    );
    assert!(
        impact.report.is_none(),
        "a blast's numbers ride the per-ganger wound/injury signals — the impact report is None",
    );
}
