use std::time::Duration;

use bevy::{
    MinimalPlugins,
    app::{App, Update},
    asset::AssetPlugin,
    math::Vec3,
    prelude::{Transform, Visibility},
    scene::ScenePlugin,
    time::TimeUpdateStrategy,
};
use gdtf_battle_sim::{
    prelude::{Cell, Level, SimPos},
    resolve_and_apply::HitReport,
    resolve_coarse::ShotKind,
    sample_cone::ShotDir,
    shot_fired::ShotFired,
    weapon::DamageType,
};

use super::super::{
    projectile::{
        PendingImpact, ProjectileTravel, ShotProjectile, advance_projectiles,
        spawn_shot_projectiles,
        travel::{ImpactPayload, ProjectileFlight},
    },
    roles::{DIRECTION_COUNT, DamageTypeFx, EffectRoles, IMPACT_FRAME_COUNT},
    tuning::{FxTuning, InterShotSeconds, ProjectileVelocity},
};
use crate::{FloatingCombatText, GangerSprites, Played, TileIndex, TopDownAtlases};

const TEST_VELOCITY: f32 = ProjectileVelocity::DEFAULT;

const TEST_INTER_SHOT: f32 = InterShotSeconds::DEFAULT;

fn test_anchor() -> (Cell, Level) {
    (Cell::new(0, 0), Level::new(0))
}

fn test_shooter() -> bevy::ecs::entity::Entity {
    bevy::ecs::entity::Entity::PLACEHOLDER
}

// A kinetic bolt carrying no pops and no hit report.
fn test_payload() -> ImpactPayload {
    ImpactPayload::new(
        DamageType::Kinetic,
        Vec::new(),
        test_anchor(),
        test_shooter(),
        None,
    )
}

fn pending_impacts(app: &mut App) -> Vec<PendingImpact> {
    let mut q = app.world_mut().query::<&PendingImpact>();
    q.iter(app.world()).cloned().collect()
}

fn pending_impact_count(app: &mut App) -> usize {
    let mut q = app.world_mut().query::<&PendingImpact>();
    q.iter(app.world()).count()
}

#[test]
fn projectile_travels_then_despawns_leaving_a_pending_impact() {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default(), ScenePlugin));
    let from = Vec3::new(0.0, 0.0, 0.0);
    let to = Vec3::new(100.0, 0.0, 0.0);
    let flight_seconds = from.distance(to) / TEST_VELOCITY;
    let step = Duration::from_secs_f32(flight_seconds / 4.0);
    app.insert_resource(TimeUpdateStrategy::ManualDuration(step));
    app.add_systems(Update, advance_projectiles);

    let proj = app
        .world_mut()
        .spawn((
            Transform::from_translation(from),
            Visibility::Hidden,
            ProjectileTravel::new(
                ProjectileFlight::new(from, to, ProjectileVelocity::default(), Duration::ZERO),
                test_payload(),
            ),
            ShotProjectile,
        ))
        .id();

    app.update();
    app.update();
    let mid_x = app
        .world()
        .entity(proj)
        .get::<Transform>()
        .map(|t| t.translation.x);
    assert!(
        matches!(mid_x, Some(x) if x > from.x && x < to.x),
        "mid-flight the projectile must sit BETWEEN muzzle and target (no stretch), got {mid_x:?}",
    );
    assert!(
        app.world().get_entity(proj).is_ok(),
        "the projectile must still be alive mid-flight",
    );
    assert_eq!(
        pending_impact_count(&mut app),
        0,
        "no impact may be handed off before arrival",
    );

    for _ in 0..6 {
        app.update();
    }
    assert!(
        app.world().get_entity(proj).is_err(),
        "the projectile must despawn on arrival (no lingering smear)",
    );
    let impacts = pending_impacts(&mut app);
    assert_eq!(
        impacts.len(),
        1,
        "arrival must hand off exactly one PendingImpact for FX-B",
    );
    if let Some(impact) = impacts.first() {
        assert!(
            impact.at.distance(to) < 0.01,
            "the PendingImpact must sit at the arrival (target) point",
        );
        assert_eq!(
            impact.damage,
            DamageType::Kinetic,
            "the PendingImpact must carry the shot's damage type",
        );
    }
}

#[test]
fn projectile_flies_at_a_constant_velocity_regardless_of_distance() {
    let step = Duration::from_secs_f32(0.1);
    let per_step_px = TEST_VELOCITY * step.as_secs_f32();
    assert!(
        per_step_px > 0.0,
        "the velocity must move the bolt a positive distance per step",
    );

    let origin = Vec3::ZERO;
    let near = Vec3::new(per_step_px, 0.0, 0.0);
    let far = Vec3::new(per_step_px * 3.0, 0.0, 0.0);
    let velocity = ProjectileVelocity::default();
    let mut short = ProjectileTravel::new(
        ProjectileFlight::new(origin, near, velocity, Duration::ZERO),
        test_payload(),
    );
    let mut long = ProjectileTravel::new(
        ProjectileFlight::new(origin, far, velocity, Duration::ZERO),
        test_payload(),
    );

    assert!(
        short.advance(step),
        "a one-step-distance shot must arrive after one velocity step",
    );
    assert!(
        !long.advance(step),
        "a 3x-distance shot must still be mid-flight after one step (same speed)",
    );
    assert!(
        !long.advance(step),
        "a 3x-distance shot must still be mid-flight after two steps",
    );
    assert!(
        long.advance(step),
        "a 3x-distance shot must arrive after exactly three velocity steps (constant speed)",
    );
}

#[test]
fn staggered_round_holds_at_the_muzzle_until_its_launch_delay_elapses() {
    let from = Vec3::ZERO;
    let to = Vec3::new(100.0, 0.0, 0.0);
    let launch_delay = Duration::from_secs_f32(TEST_INTER_SHOT);
    let mut bolt = ProjectileTravel::new(
        ProjectileFlight::new(from, to, ProjectileVelocity::default(), launch_delay),
        test_payload(),
    );

    assert!(
        !bolt.launched(),
        "a staggered round must not be launched before its delay elapses",
    );
    let part = Duration::from_secs_f32(TEST_INTER_SHOT / 2.0);
    assert!(
        !bolt.advance(part),
        "a partial tick (under the launch delay) must not arrive",
    );
    assert!(
        !bolt.launched(),
        "a round whose launch delay has not fully elapsed must stay parked at the muzzle",
    );
    assert!(
        (bolt.fraction() - 0.0).abs() < f32::EPSILON,
        "a parked round must not have advanced its travel fraction",
    );

    bolt.advance(Duration::from_secs_f32(TEST_INTER_SHOT));
    assert!(
        bolt.launched(),
        "once its launch delay elapses the round must launch (leave the muzzle)",
    );
}

fn stub_roles() -> EffectRoles {
    let block = DamageTypeFx {
        directions: [TileIndex::new(0); DIRECTION_COUNT],
        impact:     [TileIndex::new(0); IMPACT_FRAME_COUNT],
    };
    EffectRoles {
        bleed:           TileIndex::new(0),
        armor_break:     TileIndex::new(0),
        cover_destroyed: TileIndex::new(0),
        melee_strike:    TileIndex::new(0),
        fall_impact:     TileIndex::new(0),
        orange:          block.clone(),
        blue:            block.clone(),
        green:           block.clone(),
        purple:          block,
    }
}

#[test]
fn a_played_shot_with_no_effects_sheet_still_flies_a_bolt() {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default(), ScenePlugin));
    app.insert_resource(FxTuning::default());
    app.insert_resource(stub_roles());
    app.insert_resource(TopDownAtlases::empty());
    app.init_resource::<GangerSprites>();
    app.add_message::<Played<ShotFired>>();
    app.add_systems(Update, spawn_shot_projectiles);

    let shot = ShotFired {
        shooter:      app.world_mut().spawn_empty().id(),
        muzzle:       SimPos::new(0.0, 0.0, 0.0),
        trajectory:   ShotDir::from_direction(Vec3::new(1.0, 0.0, 0.0)),
        impact_cell:  Cell::new(4, 0),
        impact_level: Level::new(0),
        kind:         ShotKind::Miss,
        damage:       DamageType::Kinetic,
        report:       Some(HitReport::no_effect(ShotKind::Miss)),
    };
    let written = app.world_mut().write_message(Played::new(shot)).is_some();
    assert!(written, "Played<ShotFired> must be registered");
    app.update();

    let bolts = app
        .world_mut()
        .query::<&ShotProjectile>()
        .iter(app.world())
        .count();
    assert_eq!(
        bolts, 1,
        "a played shot must fly a bolt even when the effects sheet is missing — popping FCT \
         on the drain frame is the defect",
    );
    let pops = app
        .world_mut()
        .query::<&FloatingCombatText>()
        .iter(app.world())
        .count();
    assert_eq!(
        pops, 0,
        "hit FCT must not spawn on the drain frame when the sheet is missing",
    );
}
