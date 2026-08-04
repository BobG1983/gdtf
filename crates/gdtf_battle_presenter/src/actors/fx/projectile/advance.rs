//! Advance in-flight projectiles and seed impacts on arrival.

use bevy::{
    ecs::template::template,
    prelude::*,
    scene::{CommandsSceneExt, bsn},
};

use super::{
    pending::PendingImpact,
    travel::{ProjectileTravel, ShotProjectile},
};

/// Move projectiles along their path; spawn [`PendingImpact`] and despawn on arrival.
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
            continue;
        }
        visibility.set_if_neq(Visibility::Visible);
        transform.translation = travel.position();
        if arrived {
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
