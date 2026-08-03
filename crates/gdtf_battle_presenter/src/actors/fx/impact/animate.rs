//! Seed impact animations from pending impacts and advance their frames.

use bevy::{
    camera::visibility::RenderLayers,
    ecs::{message::MessageWriter, template::template},
    prelude::*,
    scene::{CommandsSceneExt, bsn, template_value},
};

use super::{
    super::{
        fct::FctSlotAllocator, projectile::spawn_pops_at_anchor, readers::fx_sprite_scaled,
        roles::EffectRoles, tuning::FxTuning,
    },
    ImpactAnimation,
    animation::{ImpactStep, impact_frame_scale},
    signal::ShotImpactResolved,
};
use crate::TopDownAtlases;

#[expect(
    clippy::too_many_arguments,
    reason = "Commands, tables, writer, allocator, and seed/playing queries are separate params"
)]
/// Turn pending impacts into animated flashes and emit [`ShotImpactResolved`].
pub fn animate_impact(
    mut commands: Commands,
    time: Res<Time>,
    roles: Res<EffectRoles>,
    atlases: Res<TopDownAtlases>,
    tuning: Res<FxTuning>,
    mut impact_resolved: MessageWriter<ShotImpactResolved>,
    allocator: FctSlotAllocator,
    seeds: Query<(Entity, &super::super::projectile::PendingImpact)>,
    mut playing: Query<(Entity, &mut Sprite, &mut ImpactAnimation)>,
) {
    let frame_seconds = tuning.impact_frame_seconds;
    for (seed_entity, impact) in &seeds {
        let fx = roles.fx_for(impact.damage);
        if let Some(tile) = fx.impact.first()
            && let Some(sprite) =
                fx_sprite_scaled(*tile, Color::WHITE, impact_frame_scale(0), &atlases)
        {
            let transform = Transform::from_translation(impact.at);
            let layers = RenderLayers::layer(crate::WORLD_RENDER_LAYER);
            let animation = ImpactAnimation::new(impact.damage, frame_seconds);
            commands.spawn_scene((
                bsn! { template(move |_| Ok(sprite.clone())) },
                template_value(transform),
                template_value(layers),
                bsn! { template(move |_| Ok(animation.clone())) },
            ));
        }
        spawn_pops_at_anchor(
            &mut commands,
            &allocator,
            &impact.pops,
            impact.anchor,
            &tuning,
        );
        impact_resolved.write(ShotImpactResolved {
            shooter: impact.shooter,
            report: impact.report.clone(),
        });
        commands.entity(seed_entity).despawn();
    }

    let delta = time.delta();
    for (entity, mut sprite, mut anim) in &mut playing {
        match anim.advance(delta) {
            ImpactStep::Showing(frame) => {
                let fx = roles.fx_for(anim.damage());
                if let Some(tile) = fx.impact.get(frame)
                    && let Some(next) =
                        fx_sprite_scaled(*tile, Color::WHITE, impact_frame_scale(frame), &atlases)
                {
                    *sprite = next;
                }
            }
            ImpactStep::Finished => {
                commands.entity(entity).despawn();
            }
        }
    }
}
