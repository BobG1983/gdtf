use bevy::app::App;
use gdtf_battle_presenter::{DrawnLife, GangerSprite, GangerSprites};
use gdtf_battle_sim::{
    armor::BodyPart,
    prelude::{BattleInProgress, Cell, Level, LifeState, SimPos},
    resolve_coarse::ShotKind,
    sample_cone::ShotDir,
    severity::Severity,
    shot_fired::ShotFired,
    weapon::DamageType,
};

use super::{harness::*, probes::*};

fn ganger_sprite_alive(app: &App, sim: bevy::ecs::entity::Entity) -> bool {
    app.world()
        .get_resource::<GangerSprites>()
        .is_some_and(|sprites| sprites.contains(sim))
}

fn set_life_state(app: &mut App, sim: bevy::ecs::entity::Entity, state: LifeState) {
    let mut q = app.world_mut().query::<&mut LifeState>();
    if let Ok(mut life) = q.get_mut(app.world_mut(), sim) {
        *life = state;
    }
}

fn draw_dead(app: &mut App, sim: bevy::ecs::entity::Entity) {
    app.world_mut()
        .entity_mut(sim)
        .insert(DrawnLife::new(LifeState::Dead));
}

/// A shot that kills a ganger must keep the sprite until the killing impact lands.
#[test]
fn a_shot_kill_keeps_the_sprite_until_the_killing_impact_lands() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);
    app.world_mut().insert_resource(BattleInProgress);

    let cell = Cell::new(8, 5);
    let level = Level::new(0);
    let victim = spawn_sim_ganger_with_sprite(&mut app, cell, level);
    assert!(
        ganger_sprite_alive(&app, victim),
        "the victim's presenter sprite must be spawned + mapped before the shot",
    );

    let muzzle = SimPos::new(4.0, 5.0, 0.0);
    let report = ganger_hit_report(
        victim,
        BodyPart::Torso,
        9,
        6,
        Severity::Critical,
        LifeState::Dead,
    );
    let shot = ShotFired {
        shooter: app.world_mut().spawn_empty().id(),
        muzzle,
        trajectory: ShotDir::from_direction(bevy::math::Vec3::new(1.0, 0.0, 0.0)),
        impact_cell: cell,
        impact_level: level,
        kind: ShotKind::Ganger(victim),
        damage: DamageType::Kinetic,
        report: Some(report),
    };
    play(&mut app, shot);

    set_life_state(&mut app, victim, LifeState::Dead);
    fire_with_zero_delta(&mut app);

    // At the drain frame the sprite must still be alive — its killing tracer has not landed yet.
    assert!(
        ganger_sprite_alive(&app, victim),
        "a shot-killed ganger's sprite must STILL exist at the sim-drain frame",
    );

    step_app(&mut app, std::time::Duration::from_millis(10), 1);
    assert!(
        ganger_sprite_alive(&app, victim),
        "the sprite must stay alive while the killing bolt is still in flight",
    );

    step_app(&mut app, std::time::Duration::from_millis(50), 8);
    assert!(
        !ganger_sprite_alive(&app, victim),
        "once the killing shot's impact resolves the sprite must be despawned (drop its map entry)",
    );
    let mut q = app.world_mut().query::<&GangerSprite>();
    let mirrors_victim = q.iter(app.world()).any(|marker| marker.entity == victim);
    assert!(
        !mirrors_victim,
        "no GangerSprite entity may still mirror the killed victim after the impact despawn",
    );
}

#[test]
fn a_non_shot_death_despawns_the_sprite_promptly() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);
    app.world_mut().insert_resource(BattleInProgress);

    let cell = Cell::new(3, 3);
    let level = Level::new(0);
    let dying = spawn_sim_ganger_with_sprite(&mut app, cell, level);
    assert!(
        ganger_sprite_alive(&app, dying),
        "the ganger's presenter sprite must be spawned + mapped before it dies",
    );

    set_life_state(&mut app, dying, LifeState::Dead);
    draw_dead(&mut app, dying);
    app.update();

    assert!(
        !ganger_sprite_alive(&app, dying),
        "a non-shot death (no pending tracer) must despawn the sprite promptly at the sim event",
    );
    let mut q = app.world_mut().query::<&GangerSprite>();
    let mirrors_dead = q.iter(app.world()).any(|marker| marker.entity == dying);
    assert!(
        !mirrors_dead,
        "no GangerSprite entity may still mirror the non-shot-dead ganger",
    );
}
