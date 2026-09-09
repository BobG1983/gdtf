use bevy::{app::App, sprite::Sprite, transform::components::Transform};
use gdtf_battle_presenter::{
    GangerSprites, ProjectileTravel, ShotProjectile, cell_to_world, sim_pos_to_world,
};
use gdtf_battle_sim::{
    prelude::{BattleInProgress, Cell, Level, SimPos},
    resolve_coarse::ShotKind,
    sample_cone::ShotDir,
    shot_fired::ShotFired,
    weapon::DamageType,
};

use super::{harness::*, probes::*};

fn projectile_positions_and_indices(app: &mut App) -> Vec<(bevy::math::Vec3, Option<usize>)> {
    let mut q = app
        .world_mut()
        .query::<(&ShotProjectile, &Sprite, &Transform)>();
    q.iter(app.world())
        .map(|(_, sprite, transform)| {
            (
                transform.translation,
                sprite.texture_atlas.as_ref().map(|atlas| atlas.index),
            )
        })
        .collect()
}

#[test]
fn shot_fired_spawns_directional_projectile_and_no_muzzle_flash() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);
    app.world_mut().insert_resource(BattleInProgress);

    let roles = effect_roles(&app);
    assert!(roles.is_some(), "EffectRoles must be resident");
    let Some(roles) = roles else { return };

    let struck = app.world_mut().spawn_empty().id();
    let muzzle = SimPos::new(2.0, 5.0, 0.0);
    let impact_cell = Cell::new(8, 5);
    let impact_level = Level::new(0);
    let shot = ShotFired {
        shooter: app.world_mut().spawn_empty().id(),
        muzzle,
        trajectory: ShotDir::from_direction(bevy::math::Vec3::new(1.0, 0.0, 0.0)),
        impact_cell,
        impact_level,
        kind: ShotKind::Ganger(struck),
        damage: DamageType::Kinetic,
        report: None,
    };

    play(&mut app, shot);
    fire_with_zero_delta(&mut app);

    assert_eq!(
        fx_count(&mut app),
        0,
        "a ShotFired must spawn NO muzzle FxFlash on fire (the muzzle flash was removed)",
    );

    let projectiles = projectile_positions_and_indices(&mut app);
    assert_eq!(
        projectiles.len(),
        1,
        "a ShotFired must spawn exactly one traveling projectile",
    );
    if let Some((pos, index)) = projectiles.first() {
        assert!(
            pos.distance(sim_pos_to_world(muzzle)) < 0.01,
            "the projectile must START at the muzzle world position",
        );
        assert_eq!(
            *index,
            Some(*roles.orange.directions[0]),
            "the Kinetic east shot must use the orange row's column-0 (E) directional tile",
        );
    }
}

#[test]
fn shot_fired_miss_still_spawns_projectile_and_no_muzzle_flash() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);
    app.world_mut().insert_resource(BattleInProgress);

    let shot = ShotFired {
        shooter:      app.world_mut().spawn_empty().id(),
        muzzle:       SimPos::new(1.0, 1.0, 0.0),
        trajectory:   ShotDir::from_direction(bevy::math::Vec3::new(0.0, 1.0, 0.0)),
        impact_cell:  Cell::new(1, 9),
        impact_level: Level::new(0),
        kind:         ShotKind::Miss,
        damage:       DamageType::Kinetic,
        report:       None,
    };

    play(&mut app, shot);
    app.update();

    assert_eq!(
        fx_count(&mut app),
        0,
        "a miss draws NO muzzle flash on fire (the muzzle flash was removed)",
    );
    assert_eq!(
        projectile_positions_and_indices(&mut app).len(),
        1,
        "a miss still draws a traveling projectile (it terminates at the impact cell)",
    );
}

fn ganger_sprite_world(app: &App, sim: bevy::ecs::entity::Entity) -> Option<bevy::math::Vec3> {
    let sprite_entity = app
        .world()
        .get_resource::<GangerSprites>()
        .and_then(|sprites| sprites.sprite_for(sim))?;
    app.world()
        .get::<Transform>(sprite_entity)
        .map(|transform| transform.translation)
}

fn single_projectile_arrival(app: &mut App) -> Option<bevy::math::Vec3> {
    let mut q = app.world_mut().query::<&ProjectileTravel>();
    let mut found: Option<bevy::math::Vec3> = None;
    for travel in q.iter(app.world()) {
        if found.is_some() {
            return None;
        }
        found = Some(travel.arrival());
    }
    found
}

#[test]
fn shot_fired_at_a_ganger_aims_at_the_hit_entitys_rendered_position() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);
    app.world_mut().insert_resource(BattleInProgress);

    let ganger_cell = Cell::new(8, 5);
    let level = Level::new(0);
    let struck = spawn_sim_ganger_with_sprite(&mut app, ganger_cell, level);

    let ganger_world = ganger_sprite_world(&app, struck);
    assert!(
        ganger_world.is_some(),
        "the struck ganger's presenter sprite must be spawned + mapped in GangerSprites",
    );
    let Some(ganger_world) = ganger_world else {
        return;
    };

    let muzzle = SimPos::new(1.0, 1.0, 0.0);
    let impact_cell = Cell::new(2, 2);
    let impact_level = Level::new(0);
    let shot = ShotFired {
        shooter: app.world_mut().spawn_empty().id(),
        muzzle,
        trajectory: ShotDir::from_direction(bevy::math::Vec3::new(1.0, 0.0, 0.0)),
        impact_cell,
        impact_level,
        kind: ShotKind::Ganger(struck),
        damage: DamageType::Kinetic,
        report: None,
    };
    play(&mut app, shot);
    app.update();

    let arrival = single_projectile_arrival(&mut app);
    assert!(
        arrival.is_some(),
        "exactly one traveling projectile must spawn for the ganger hit",
    );
    let Some(arrival) = arrival else { return };

    assert!(
        arrival.truncate().distance(ganger_world.truncate()) < 0.01,
        "the bolt must aim at the hit ganger's rendered position {ganger_world:?}, got {arrival:?}",
    );
    let impact_world = cell_to_world(impact_cell, impact_level);
    assert!(
        arrival.truncate().distance(impact_world.truncate()) > 0.01,
        "the bolt must NOT aim at the impact cell {impact_world:?} for a ganger hit (entity-aim)",
    );
}
