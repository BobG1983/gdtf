//! Traveling shot projectile: spawn-at-muzzle, no muzzle flash, entity-aim
//! (GTW-306/307).

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

/// The (world translation, atlas index) of every `ShotProjectile` sprite (the GTW-306
/// traveling projectiles), unordered.
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

/// GTW-307 — a `ShotFired` spawns NO standalone muzzle flash (it was removed: it rendered
/// oversized at the shooter's feet and read poorly), only a traveling DIRECTIONAL projectile (a
/// `ShotProjectile` starting at the muzzle, on a `ProjectileTravel` toward the impact). The
/// projectile uses the per-damage-type directional tile (orange row for Kinetic) and IS the
/// fire signal; the old stretched tracer is also gone.
#[test]
fn shot_fired_spawns_directional_projectile_and_no_muzzle_flash() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);
    app.world_mut().insert_resource(BattleInProgress);

    let roles = effect_roles(&app);
    assert!(roles.is_some(), "EffectRoles must be resident");
    let Some(roles) = roles else { return };

    // A struck ganger entity (the ShotKind::Ganger payload) + a known fire geometry, fired
    // EAST (the +x heading -> compass column 0 of the row).
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
    // Pin the firing update's clock delta to ZERO so the just-spawned projectile cannot travel a
    // wall-clock-dependent distance off the muzzle on the read frame: under `Automatic` time, the
    // first update after the variable-length `settle_resources` carries a non-deterministic delta,
    // which flaked the "starts at the muzzle" assertion under parallel `cargo` load (GTW-305). A
    // zero delta keeps `advance_projectiles` from moving the bolt regardless of scheduling, without
    // weakening the assertion (the spawn-at-muzzle truth it pins is delta-independent).
    fire_with_zero_delta(&mut app);

    // No standalone muzzle FxFlash spawns on fire (the muzzle flash was removed in GTW-307;
    // the impact-frame flashes spawn later, when the projectile ARRIVES, not on fire).
    assert_eq!(
        fx_count(&mut app),
        0,
        "a ShotFired must spawn NO muzzle FxFlash on fire (the muzzle flash was removed)",
    );

    // Exactly one traveling directional projectile, starting at the muzzle, at the orange-row
    // east tile (Kinetic -> orange; +x heading -> column 0).
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

/// GTW-307 — a MISS still draws the firing FX: a traveling projectile (the projectile
/// terminates at the impact cell). A miss carries no struck object but the same geometry, so
/// the presenter draws the same projectile — and, like a hit, NO standalone muzzle flash.
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

/// The `GangerSprites`-mapped presenter sprite's rendered world `Transform.translation` for
/// `sim` ganger, or `None` if it was not spawned/mapped (the caller asserts `Some`).
fn ganger_sprite_world(app: &App, sim: bevy::ecs::entity::Entity) -> Option<bevy::math::Vec3> {
    let sprite_entity = app
        .world()
        .get_resource::<GangerSprites>()
        .and_then(|sprites| sprites.sprite_for(sim))?;
    app.world()
        .get::<Transform>(sprite_entity)
        .map(|transform| transform.translation)
}

/// The arrival (target) world point of the SINGLE `ShotProjectile` in flight — its
/// `ProjectileTravel` terminus. Returns `None` unless exactly one projectile exists (the
/// caller asserts `Some`).
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

/// GTW-306 (C2 / C5) — a `ShotFired` that struck a GANGER aims its traveling projectile at
/// that hit entity's CURRENT rendered world position (its presenter `Transform`, looked up
/// through `GangerSprites`), NOT at the impact `(cell, level)`. This drives the REAL
/// `spawn_ganger_sprites` path so the hit ganger has a mapped sprite, places that ganger in a
/// DIFFERENT cell from the message's `impact_cell`, and asserts the projectile's
/// `ProjectileTravel` terminus equals the ganger sprite's rendered translation — and is
/// distinctly NOT `cell_to_world(impact_cell, impact_level)`. Reverting the entity-aim logic
/// to always use `cell_to_world(impact_cell)` would flip both assertions (the path is no
/// longer dead-code to the suite).
#[test]
fn shot_fired_at_a_ganger_aims_at_the_hit_entitys_rendered_position() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);
    app.world_mut().insert_resource(BattleInProgress);

    // The struck ganger sits at a cell DISTINCT from the shot's impact cell, so the
    // entity-aim endpoint and the impact-cell fallback are unambiguously different points.
    let ganger_cell = Cell::new(8, 5);
    let level = Level::new(0);
    let struck = spawn_sim_ganger_with_sprite(&mut app, ganger_cell, level);

    // The ganger's sprite must have been spawned + mapped by the real spawn system.
    let ganger_world = ganger_sprite_world(&app, struck);
    assert!(
        ganger_world.is_some(),
        "the struck ganger's presenter sprite must be spawned + mapped in GangerSprites",
    );
    let Some(ganger_world) = ganger_world else {
        return;
    };

    // Fire AT that ganger, but with an impact cell DELIBERATELY elsewhere — proving the bolt
    // tracks the entity, not the message's impact cell.
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

    // The projectile must fly to the hit ganger's RENDERED position (entity-aim), the (x, y)
    // of its sprite Transform — NOT the impact cell. (Compare x/y: the projectile's z is the
    // FX layer, distinct from the ganger sprite's Actor-layer z bias, so a 2D compare isolates
    // the aim choice from the z-layering.)
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
