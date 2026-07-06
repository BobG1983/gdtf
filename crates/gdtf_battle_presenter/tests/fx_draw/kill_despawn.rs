//! Kill-despawn deferral to the killing impact (GTW-331).

use bevy::{app::App, ecs::message::Messages};
use gdtf_battle_presenter::{GangerSprite, GangerSprites};
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

/// Whether sim ganger `sim` currently has a LIVE presenter sprite — its `GangerSprites` entry is
/// still mapped (GTW-331; the despawn drops the map entry, so a missing entry == despawned).
fn ganger_sprite_alive(app: &App, sim: bevy::ecs::entity::Entity) -> bool {
    app.world()
        .get_resource::<GangerSprites>()
        .is_some_and(|sprites| sprites.contains(sim))
}

/// Set the `LifeState` of sim ganger `sim` directly (the sim-drain `Changed<LifeState>` trigger
/// the presenter reacts to). GTW-331: this is the moment the bug despawned the sprite — long
/// before the killing tracer lands.
fn set_life_state(app: &mut App, sim: bevy::ecs::entity::Entity, state: LifeState) {
    let mut q = app.world_mut().query::<&mut LifeState>();
    if let Ok(mut life) = q.get_mut(app.world_mut(), sim) {
        *life = state;
    }
}

/// GTW-331 — the BUG FIX, deterministic + headless: a SHOT that KILLS a ganger must keep the
/// ganger's sprite ALIVE at sim-drain time (when the sim flips its `LifeState` to `Dead`, well
/// before the staggered killing tracer arrives) and despawn it ONLY after the killing shot's
/// impact resolves (when the bolt reaches the body).
///
/// This drives the FULL FX pipeline: the real `spawn_ganger_sprites` registers the victim's
/// sprite, a `ShotFired` carrying a lethal (`life_after == Dead`) ganger-hit report spawns the
/// bolt, and the victim's `LifeState` is set `Dead` on the SAME drain frame (the sim-drain the
/// bug reacted to). The assertions: (1) at the drain frame the sprite is STILL alive (the bug:
/// it was already despawned), (2) the bolt is still in flight so it stays alive, (3) once the
/// bolt flies to its impact and `animate_impact` resolves the kill, the sprite is despawned and
/// its `GangerSprites` entry dropped.
///
/// RED before the fix: `update_ganger_life_state`'s old `Dead` arm despawned at the drain frame,
/// so assertion (1) (the sprite still alive at drain) fails immediately.
#[test]
fn a_shot_kill_keeps_the_sprite_until_the_killing_impact_lands() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);
    app.world_mut().insert_resource(BattleInProgress);

    // The struck ganger sits at a cell a short flight from the muzzle, with a REAL presenter
    // sprite registered by the production spawn system.
    let cell = Cell::new(8, 5);
    let level = Level::new(0);
    let victim = spawn_sim_ganger_with_sprite(&mut app, cell, level);
    assert!(
        ganger_sprite_alive(&app, victim),
        "the victim's presenter sprite must be spawned + mapped before the shot",
    );

    // A lethal (DEAD) penetrating torso hit on the victim, fired from one cell west — a short
    // flight, so the bolt takes several updates to reach the body.
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
    app.world_mut()
        .resource_mut::<Messages<ShotFired>>()
        .write(shot);

    // The sim-drain frame: drain the ShotFired (spawn the bolt, still parked/short into flight)
    // AND flip the victim's LifeState to Dead, the SAME frame — exactly what the sim does at
    // drain. The bug despawned the sprite here.
    set_life_state(&mut app, victim, LifeState::Dead);
    fire_with_zero_delta(&mut app);

    // (1) THE BUG: at the drain frame the sprite must STILL be alive — its killing tracer has
    // not reached it yet.
    assert!(
        ganger_sprite_alive(&app, victim),
        "a shot-killed ganger's sprite must STILL exist at the sim-drain frame (the bug despawned \
         it here, before the killing tracer arrives)",
    );

    // (2) The bolt is in flight; a couple of small steps keep it short of the impact, so the
    // sprite stays alive across the flight (the kill is pending the incoming impact).
    step_app(&mut app, std::time::Duration::from_millis(10), 1);
    assert!(
        ganger_sprite_alive(&app, victim),
        "the sprite must stay alive while the killing bolt is still in flight",
    );

    // (3) Fly the bolt the rest of the way to its impact: animate_impact resolves the kill, and
    // the sprite is despawned + its map entry dropped — at the IMPACT, not at the drain.
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

/// GTW-331 — the fix must NOT drop a NON-shot death: a ganger that dies WITHOUT a tracer (its
/// `LifeState` set `Dead` directly — a bleed-out / direct kill, no `ShotFired`, no projectile)
/// must STILL have its sprite despawned PROMPTLY at the sim event (there is no incoming impact to
/// wait for). Guards the deferral from silently dropping deaths that have no killing shot.
#[test]
fn a_non_shot_death_despawns_the_sprite_promptly() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);
    app.world_mut().insert_resource(BattleInProgress);

    // A real presenter sprite, no shot fired at all (no projectile in flight, no PendingImpact).
    let cell = Cell::new(3, 3);
    let level = Level::new(0);
    let dying = spawn_sim_ganger_with_sprite(&mut app, cell, level);
    assert!(
        ganger_sprite_alive(&app, dying),
        "the ganger's presenter sprite must be spawned + mapped before it dies",
    );

    // A non-shot death: set LifeState Dead directly (a bleed-out-style death). No tracer is
    // pending, so the life-state path must despawn it on the next update.
    set_life_state(&mut app, dying, LifeState::Dead);
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
