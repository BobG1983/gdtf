//! Unit tests for the transient FX layer — flashes (S6/GTW-220) + the GTW-306
//! per-damage-type traveling projectile.

use std::time::Duration;

use bevy::{
    MinimalPlugins,
    app::{App, Update},
    asset::AssetPlugin,
    math::Vec3,
    prelude::{Alpha, Transform, Visibility},
    scene::ScenePlugin,
    time::TimeUpdateStrategy,
};
use gdtf_battle_sim::{Cell, DamageType, Level, Wounds};

use super::{
    flash::{FLASH_SECONDS, FlashTtl},
    projectile::{PendingImpact, ProjectileTravel, ShotProjectile, advance_projectiles},
    readers::bleed_tint,
    roles::{COMPASS_DIRECTIONS, DIRECTION_COUNT, EffectRoles, nearest_direction_index},
    tuning::{InterShotSeconds, ProjectileVelocity},
};

/// The constant flight speed (px/sec) the projectile unit tests drive `advance` with — the
/// shipped hot-reloadable default (what the resident `FxTuning` carries with no `.ron`
/// override), so the tests pin the same speed the system uses.
const TEST_VELOCITY: f32 = ProjectileVelocity::DEFAULT;

/// The burst stagger step (seconds) the stagger unit test drives a round's launch delay with —
/// the shipped hot-reloadable default.
const TEST_INTER_SHOT: f32 = InterShotSeconds::DEFAULT;

/// A throw-away anchor `(cell, level)` for the projectile-FLIGHT unit tests, which exercise the
/// muzzle→target travel + arrival seam, not the GTW-327 FCT pops (those rides empty in these
/// tests; the pop-staggering is proven on the real registered-system path in `fx_draw.rs`).
fn test_anchor() -> (Cell, Level) {
    (Cell::new(0, 0), Level::new(0))
}

/// A throw-away firing entity for the projectile-FLIGHT unit tests — the GTW-328 shooter the bolt
/// threads to its `PendingImpact`; these flight-only tests exercise the travel + arrival seam, not
/// the shot-impact signal (its staggering is proven on the real path in `fx_draw.rs`).
fn test_shooter() -> bevy::ecs::entity::Entity {
    bevy::ecs::entity::Entity::PLACEHOLDER
}

/// The shipped `effect_roles.ron` parses into `EffectRoles` and exposes every FX role —
/// a `ron::de` round-trip of the SHIPPED bytes.
///
/// It asserts the file PARSES and HAS all roles (a missing field is a deserialize error);
/// it does NOT pin a tunable index magnitude (those are data the engineer eyeballs and may
/// adjust). A light distinctness guard catches an all-collapsed authoring slip — the four
/// damage-type rows must not share one directional strip.
#[test]
fn shipped_effect_roles_ron_parses_with_all_roles() {
    const SHIPPED: &str = include_str!("../../../../../assets/tiles/effect_roles.ron");
    let parsed: Result<EffectRoles, _> = ron::de::from_str(SHIPPED);
    assert!(
        parsed.is_ok(),
        "shipped effect_roles.ron must parse into EffectRoles, got: {:?}",
        parsed.as_ref().err(),
    );
    let Ok(roles) = parsed else {
        return;
    };
    // The three consequence roles must not all collapse onto one index (an authoring slip).
    let conseq_same =
        roles.bleed == roles.armor_break && roles.armor_break == roles.cover_destroyed;
    assert!(
        !conseq_same,
        "the consequence FX roles must not all share one index (authoring slip)",
    );
    // The four damage-type rows must each carry a DISTINCT directional strip (a per-type
    // color variant) — an all-collapsed authoring slip would make every shot look alike.
    let rows = [
        roles.orange.directions,
        roles.blue.directions,
        roles.green.directions,
        roles.purple.directions,
    ];
    for (i, a) in rows.iter().enumerate() {
        for b in rows.iter().skip(i + 1) {
            assert_ne!(
                a, b,
                "each damage-type row must carry its own directional strip (a per-type color)",
            );
        }
    }
}

/// Each `DamageType` maps to a per-type FX row, the row carries a full 8-way directional
/// strip + a 3-frame impact, and the sweep is total over `DamageType::ALL` — the per-type
/// MECHANISM the contract requires built across the enum (only Kinetic ships in data today).
#[test]
fn fx_for_resolves_every_damage_type_to_a_full_row() {
    const SHIPPED: &str = include_str!("../../../../../assets/tiles/effect_roles.ron");
    let parsed: Result<EffectRoles, _> = ron::de::from_str(SHIPPED);
    assert!(
        parsed.is_ok(),
        "shipped effect_roles.ron must parse, got: {:?}",
        parsed.as_ref().err(),
    );
    let Ok(roles) = parsed else {
        return;
    };
    for damage in DamageType::ALL {
        let fx = roles.fx_for(damage);
        assert_eq!(
            fx.directions.len(),
            DIRECTION_COUNT,
            "{damage:?} must resolve to a full 8-way directional strip",
        );
        assert_eq!(
            fx.impact.len(),
            3,
            "{damage:?} must resolve to a 3-frame impact strip",
        );
    }
    // Kinetic (the only type in skirmish data) reads the orange row; the fallback is orange.
    assert_eq!(
        roles.fx_for(DamageType::Kinetic),
        &roles.orange,
        "Kinetic (the in-data type) must read the orange row",
    );
    assert_eq!(
        roles.fallback(),
        &roles.orange,
        "the fallback row must be the orange row",
    );
}

/// `nearest_direction_index` picks the compass column whose heading the trajectory points
/// closest to — each cardinal/diagonal heading resolves to its OWN column, and a column's
/// own heading is its own nearest (round-trip identity).
#[test]
fn nearest_direction_index_picks_the_matching_compass_column() {
    // Each authored compass column's own heading must resolve back to that column.
    for (index, dir) in COMPASS_DIRECTIONS.iter().enumerate() {
        let picked = nearest_direction_index(Vec3::new(dir.x, dir.y, 0.0));
        assert_eq!(
            picked, index,
            "compass column {index}'s own heading must pick column {index}, got {picked}",
        );
    }
    // A z-only (straight up/down) trajectory has no XY heading -> defaults to column 0 (E).
    assert_eq!(
        nearest_direction_index(Vec3::new(0.0, 0.0, 1.0)),
        0,
        "a straight-up shot (no XY heading) must default to column 0",
    );
    // The picked index is always a valid strip column.
    assert!(
        nearest_direction_index(Vec3::new(0.3, -0.9, 0.2)) < DIRECTION_COUNT,
        "the picked direction index must be a valid strip column",
    );
}

/// A traveling projectile flies muzzle→target at a CONSTANT VELOCITY under
/// `advance_projectiles`: it starts at the muzzle, moves toward the target as time advances,
/// and despawns + leaves a `PendingImpact` at the arrival point once it has flown the whole
/// distance. A zero launch delay (a single shot) launches at once.
#[test]
fn projectile_travels_then_despawns_leaving_a_pending_impact() {
    let mut app = App::new();
    // GTW-322: `advance_projectiles`' arrival hands off the `PendingImpact` via
    // `commands.spawn_scene(bsn! { .. })`, which panics without the `AssetPlugin` + `ScenePlugin`
    // the deferred `apply_scene` reads — so the harness adds both. The `PendingImpact` then
    // materializes on the `SpawnScene` schedule (run inside each `app.update()` below), so the
    // post-arrival update loop already drives it before the count assertion.
    app.add_plugins((MinimalPlugins, AssetPlugin::default(), ScenePlugin));
    // A small controlled per-update delta — a fraction of the flight, so the FIRST measured
    // update lands the projectile MID-flight (the virtual clock reports 0 on the warm-up
    // update, then this fixed step each subsequent one). The 100px flight at TEST_VELOCITY
    // takes ~100/TEST_VELOCITY s; a quarter-of-flight step lands one update mid-air.
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
            // Production spawns every round Hidden-until-launch (the GTW-308 stagger reveal),
            // so `advance_projectiles`' query takes `&mut Visibility`; the spawned projectile
            // must carry it to match the real query (a zero launch delay reveals it at once).
            Visibility::Hidden,
            // A single shot — zero launch delay launches at once, flying at the default velocity.
            // No FCT pops in this flight-only test (the GTW-327 pop staggering is proven on the
            // real path in fx_draw.rs).
            ProjectileTravel::new(
                from,
                to,
                DamageType::Kinetic,
                ProjectileVelocity::default(),
                Duration::ZERO,
                Vec::new(),
                test_anchor(),
                test_shooter(),
                None,
            ),
            ShotProjectile,
        ))
        .id();

    // Warm-up update (the virtual clock's first-step zero delta) — no progress yet.
    app.update();
    // One real step: the projectile must sit BETWEEN muzzle and target, still alive.
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

    // Drive the rest of the flight to completion (each step a quarter; a few more finishes it).
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

/// Every shot flies at the SAME constant px/sec (GTW-306 velocity, not a fixed-seconds
/// window): a flight TWICE as long takes TWICE as many velocity steps to arrive. Driven off
/// the same `advance` stepper `advance_projectiles` runs, so it pins the velocity contract: a
/// near and a far shot share one visual speed.
#[test]
fn projectile_flies_at_a_constant_velocity_regardless_of_distance() {
    // A 1/10th-second step at the shipped velocity covers a known px/step.
    let step = Duration::from_secs_f32(0.1);
    let per_step_px = TEST_VELOCITY * step.as_secs_f32();
    assert!(
        per_step_px > 0.0,
        "the velocity must move the bolt a positive distance per step",
    );

    // A short flight (one step's worth of distance) and a long flight (three steps' worth),
    // both from a zero launch delay (launched at once), flying at the default velocity.
    let origin = Vec3::ZERO;
    let near = Vec3::new(per_step_px, 0.0, 0.0);
    let far = Vec3::new(per_step_px * 3.0, 0.0, 0.0);
    let velocity = ProjectileVelocity::default();
    let mut short = ProjectileTravel::new(
        origin,
        near,
        DamageType::Kinetic,
        velocity,
        Duration::ZERO,
        Vec::new(),
        test_anchor(),
        test_shooter(),
        None,
    );
    let mut long = ProjectileTravel::new(
        origin,
        far,
        DamageType::Kinetic,
        velocity,
        Duration::ZERO,
        Vec::new(),
        test_anchor(),
        test_shooter(),
        None,
    );

    // The near shot arrives in ONE velocity step.
    assert!(
        short.advance(step),
        "a one-step-distance shot must arrive after one velocity step",
    );
    // The far shot is 3x the distance, so it needs THREE steps — the SAME px/sec speed.
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

/// The stagger (GTW-308): a round's launch delay HOLDS it at the muzzle until the delay
/// elapses, then it flies. A round-1 bolt (one `INTER_SHOT_SECONDS` late) does not launch (stays
/// un`launched`, no travel) until its delay passes — so a burst reads shot-by-shot. Driven off
/// the `advance`/`launched` pair `advance_projectiles` reads.
#[test]
fn staggered_round_holds_at_the_muzzle_until_its_launch_delay_elapses() {
    let from = Vec3::ZERO;
    let to = Vec3::new(100.0, 0.0, 0.0);
    // Round 1 of a burst: a one-step launch delay.
    let launch_delay = Duration::from_secs_f32(TEST_INTER_SHOT);
    let mut bolt = ProjectileTravel::new(
        from,
        to,
        DamageType::Kinetic,
        ProjectileVelocity::default(),
        launch_delay,
        Vec::new(),
        test_anchor(),
        test_shooter(),
        None,
    );

    // Before the launch delay elapses the bolt is parked at the muzzle — not launched, fraction 0.
    assert!(
        !bolt.launched(),
        "a staggered round must not be launched before its delay elapses",
    );
    // A tick SHORTER than the launch delay keeps it parked (still not launched, no arrival).
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

    // Once the launch delay has fully elapsed it launches and starts flying.
    bolt.advance(Duration::from_secs_f32(TEST_INTER_SHOT));
    assert!(
        bolt.launched(),
        "once its launch delay elapses the round must launch (leave the muzzle)",
    );
}

/// The `PendingImpact`s currently in the world (FX-B's impact-animation seeds).
///
/// Cloned (not `.copied()`): `PendingImpact` carries the GTW-327 owned pop `Vec`, so it is no
/// longer `Copy`.
fn pending_impacts(app: &mut App) -> Vec<PendingImpact> {
    let mut q = app.world_mut().query::<&PendingImpact>();
    q.iter(app.world()).cloned().collect()
}

/// How many `PendingImpact`s are in the world.
fn pending_impact_count(app: &mut App) -> usize {
    let mut q = app.world_mut().query::<&PendingImpact>();
    q.iter(app.world()).count()
}

/// `bleed_tint` is a strictly-DECREASING relation in remaining wounds: a ganger nearer
/// death (fewer wounds) bleeds a more opaque flash, never a pinned literal.
#[test]
fn bleed_tint_alpha_decreases_with_remaining_wounds() {
    let near_death = bleed_tint(Wounds::new(0)).alpha();
    let healthier = bleed_tint(Wounds::new(5)).alpha();
    assert!(
        near_death > healthier,
        "fewer remaining wounds must bleed a MORE opaque (higher alpha) flash: \
         {near_death} (0 wounds) must exceed {healthier} (5 wounds)",
    );
}

/// A fresh `FlashTtl` is not finished, and ticking it past `FLASH_SECONDS` finishes it —
/// the one-shot countdown `expire_flashes` keys its despawn on.
#[test]
fn flash_ttl_finishes_after_its_window() {
    let mut ttl = FlashTtl::new();
    // A zero tick does not finish a fresh one-shot timer.
    assert!(
        !ttl.tick(Duration::ZERO),
        "a fresh FlashTtl must not be finished before any time passes",
    );
    // Ticking past the full window finishes it.
    let past = Duration::from_secs_f32(FLASH_SECONDS + 0.1);
    assert!(
        ttl.tick(past),
        "ticking a FlashTtl past FLASH_SECONDS must finish it (the one-shot signal)",
    );
}
