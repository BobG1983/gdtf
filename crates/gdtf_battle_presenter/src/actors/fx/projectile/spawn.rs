//! Spawn shot projectiles from played `ShotFired` messages.

use bevy::{
    camera::visibility::RenderLayers,
    ecs::template::template,
    prelude::*,
    scene::{CommandsSceneExt, bsn, template_value},
};
use gdtf_battle_sim::{
    prelude::{Cell, CellLevel, Level, Position},
    resolve_coarse::ShotKind,
    shot_fired::ShotFired,
};

use super::{
    super::{
        fct::{
            ClassifiedPop, FctSlotAllocator, FctStackIndex, anchor_cell, classify_report,
            spawn_floating_text,
        },
        readers::fx_sprite_scaled,
        roles::{EffectRoles, nearest_direction_index},
        tuning::FxTuning,
    },
    travel::{ProjectileTravel, ShotProjectile},
};
use crate::{
    GangerSprite, GangerSprites, TopDownAtlases, cell_to_world, playback::Played, sim_pos_to_world,
};

#[expect(
    clippy::too_many_arguments,
    reason = "atlases, roles, aim lookups, anchor query, allocator, and ShotFired reader are separate params"
)]
/// Spawn a directional projectile (or immediate pops if the sheet is missing).
pub fn spawn_shot_projectiles(
    mut commands: Commands,
    atlases: Res<TopDownAtlases>,
    roles: Res<EffectRoles>,
    tuning: Res<FxTuning>,
    ganger_sprites: Res<GangerSprites>,
    ganger_transforms: Query<&Transform, With<GangerSprite>>,
    positions: Query<&Position>,
    allocator: FctSlotAllocator,
    mut shots: MessageReader<Played<ShotFired>>,
) {
    let draw_scale = *tuning.projectile_draw_scale;
    let velocity = tuning.projectile_velocity;
    for msg in shots.read() {
        let muzzle_world = sim_pos_to_world(msg.muzzle);
        let target_world = ganger_hit_world(msg.kind, &ganger_sprites, &ganger_transforms)
            .unwrap_or_else(|| cell_to_world(msg.impact_cell, msg.impact_level));

        let pops = classify_report(msg.report.as_ref());
        let anchor = anchor_cell(msg, &positions);

        let fx = roles.fx_for(msg.damage);
        let dir_index = nearest_direction_index(msg.trajectory.vec());
        let tile = fx.directions.get(dir_index);
        let sprite = tile.and_then(|t| fx_sprite_scaled(*t, Color::WHITE, draw_scale, &atlases));
        let Some(sprite) = sprite else {
            spawn_pops_at_anchor(&mut commands, &allocator, &pops, anchor, &tuning);
            continue;
        };
        let launch_delay = std::time::Duration::ZERO;
        let travel = ProjectileTravel::new(
            muzzle_world,
            target_world,
            msg.damage,
            velocity,
            launch_delay,
            pops,
            anchor,
            msg.shooter,
            msg.report.clone(),
        );
        let transform = Transform::from_translation(muzzle_world);
        let layers = RenderLayers::layer(crate::WORLD_RENDER_LAYER);
        commands
            .spawn_scene((
                bsn! { template(move |_| Ok(sprite.clone())) },
                template_value(transform),
                template_value(Visibility::Hidden),
                template_value(layers),
                bsn! { template(move |_| Ok(travel.clone())) },
            ))
            .insert(ShotProjectile);
    }
}

pub(in crate::actors::fx) fn spawn_pops_at_anchor(
    commands: &mut Commands,
    allocator: &FctSlotAllocator,
    pops: &[ClassifiedPop],
    anchor: (Cell, Level),
    tuning: &FxTuning,
) {
    let (cell, level) = anchor;
    let base = *allocator.next_slot(CellLevel::new(cell, level));
    for (offset, pop) in pops.iter().enumerate() {
        spawn_floating_text(
            commands,
            pop.text().clone(),
            pop.color(),
            pop.emphasis(),
            cell,
            level,
            FctStackIndex::new(base + offset),
            tuning.fct_ttl_seconds,
            tuning.fct_rise_rate,
        );
    }
}

fn ganger_hit_world(
    kind: ShotKind,
    ganger_sprites: &GangerSprites,
    ganger_transforms: &Query<&Transform, With<GangerSprite>>,
) -> Option<Vec3> {
    let ShotKind::Ganger(sim_entity) = kind else {
        return None;
    };
    let sprite_entity = ganger_sprites.sprite_for(sim_entity)?;
    let transform = ganger_transforms.get(sprite_entity).ok()?;
    Some(transform.translation)
}
