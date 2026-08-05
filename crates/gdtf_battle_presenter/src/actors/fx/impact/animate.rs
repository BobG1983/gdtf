//! Seed impact animations from pending impacts and advance their frames.

use bevy::{
    camera::visibility::RenderLayers,
    ecs::{message::MessageWriter, template::template},
    prelude::*,
    scene::{CommandsSceneExt, bsn, template_value},
};

use super::{
    super::{
        fct::FctSlotAllocator, projectile::spawn_pops_at_anchor, sprites::FxSprites,
        tuning::FxTuning,
    },
    ImpactAnimation,
    animation::{ImpactStep, impact_frame_scale},
    signal::ShotImpactResolved,
};

/// Turn pending impacts into animated flashes and emit [`ShotImpactResolved`].
pub fn seed_impact_animations(
    mut commands: Commands,
    sprites: FxSprites,
    tuning: Res<FxTuning>,
    mut impact_resolved: MessageWriter<ShotImpactResolved>,
    allocator: FctSlotAllocator,
    seeds: Query<(Entity, &super::super::projectile::PendingImpact)>,
) {
    let frame_seconds = tuning.impact_frame_seconds;
    for (seed_entity, impact) in &seeds {
        let first_tile = sprites.fx_for(impact.damage).impact.first().copied();
        if let Some(tile) = first_tile
            && let Some(sprite) = sprites.tile(tile, Color::WHITE, impact_frame_scale(0))
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
            report:  impact.report.clone(),
        });
        commands.entity(seed_entity).despawn();
    }
}

/// Step playing impact flashes to their next frame; despawn when finished.
pub fn advance_impact_animations(
    mut commands: Commands,
    time: Res<Time>,
    sprites: FxSprites,
    mut playing: Query<(Entity, &mut Sprite, &mut ImpactAnimation)>,
) {
    let delta = time.delta();
    for (entity, mut sprite, mut anim) in &mut playing {
        match anim.advance(delta) {
            ImpactStep::Showing(frame) => {
                let next_tile = sprites.fx_for(anim.damage()).impact.get(frame).copied();
                if let Some(tile) = next_tile
                    && let Some(next) = sprites.tile(tile, Color::WHITE, impact_frame_scale(frame))
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
