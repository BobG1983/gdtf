//! The per-frame flight stepper + the arrival impact handoff.

use bevy::{
    ecs::template::template,
    prelude::*,
    scene::{CommandsSceneExt, bsn},
};

use super::{
    pending::PendingImpact,
    travel::{ProjectileTravel, ShotProjectile},
};

/// `Update` (`PresenterSystems::Overlay`): advance every traveling projectile and HAND OFF its
/// impact on arrival.
///
/// Advances each [`ProjectileTravel`] by the frame [`Res<Time>`] delta. A round whose staggered
/// launch delay has NOT yet elapsed is held INVISIBLE at the muzzle ([`Visibility::Hidden`], no
/// translation) so a multi-round volley animates shot-by-shot; the frame its launch delay
/// elapses it becomes [`Visibility::Visible`] and thereafter its `Transform` is written to its
/// current travel point ([`ProjectileTravel::position`]) — constant-VELOCITY TRANSLATION, the
/// sprite never stretched/scaled. The instant the bolt has flown the whole `from → to` distance
/// ([`ProjectileTravel::advance`] returns `true`), it `Commands::entity(e).despawn`s the
/// projectile AND `Commands::spawn`s a [`PendingImpact`] at the arrival point carrying the shot's
/// damage type — the seam FX-B's [`animate_impact`](super::super::impact::animate_impact) reads.
/// It
/// touches ONLY [`ShotProjectile`]-marked entities; it needs no `BattleInProgress` gate (inert
/// with no projectiles — the query is empty — so a projectile spawned during a battle still
/// completes its flight after it ends).
///
/// Param-only (`bevy-traps.md` #7): [`Commands`] for the despawn / impact spawn, [`Res<Time>`]
/// for the delta, and the `(Entity, &mut Transform, &mut Visibility, &mut ProjectileTravel)`
/// query (`With<ShotProjectile>`).
pub fn advance_projectiles(
    mut commands: Commands,
    time: Res<Time>,
    mut projectiles: Query<
        (
            Entity,
            &mut Transform,
            &mut Visibility,
            &mut ProjectileTravel,
        ),
        With<ShotProjectile>,
    >,
) {
    let delta = time.delta();
    for (entity, mut transform, mut visibility, mut travel) in &mut projectiles {
        let arrived = travel.advance(delta);
        if !travel.launched() {
            // Still parked at the muzzle — keep it hidden, do not move it yet.
            continue;
        }
        // Launched: reveal it (set_if_neq avoids a needless change-detection write each frame)
        // and translate to the current flight point (never scale — that was the bug).
        visibility.set_if_neq(Visibility::Visible);
        transform.translation = travel.position();
        if arrived {
            // Hand the impact off to FX-B at the arrival point, carrying this shot's classified
            // FCT pops + their anchor (GTW-327) so the numbers appear with THIS bolt's impact,
            // then despawn the bolt (its pops are MOVED into the impact, not duplicated).
            //
            // GTW-322 — the seam entity carries ONLY `PendingImpact`, which owns the pop `Vec`
            // (no `Default`), so it is spawned as a single-`bsn!` scene via the
            // `template(move |_| Ok(value.clone()))` closure escape hatch (the bare entity the
            // old `commands.spawn(PendingImpact { .. })` produced — same component, deferred to
            // that frame's `SpawnScene` schedule).
            let pending = PendingImpact {
                at:      travel.arrival(),
                damage:  travel.damage(),
                anchor:  travel.anchor(),
                shooter: travel.shooter(),
                report:  travel.report(),
                pops:    travel.take_pops(),
            };
            commands.spawn_scene(bsn! { template(move |_| Ok(pending.clone())) });
            commands.entity(entity).despawn();
        }
    }
}
