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
            ClassifiedPop, FctDrift, FctSlot, FctSlotAllocator, FctStackIndex, anchor_cell,
            classify_report, spawn_floating_text,
        },
        roles::nearest_direction_index,
        sprites::FxSprites,
        tuning::FxTuning,
    },
    travel::{ImpactPayload, ProjectileFlight, ProjectileTravel, ShotProjectile},
};
use crate::{GangerSpriteWorld, cell_to_world, playback::Played, sim_pos_to_world};

/// Spawn a directional projectile (or immediate pops if the sheet is missing).
pub fn spawn_shot_projectiles(
    mut commands: Commands,
    sprites: FxSprites,
    tuning: Res<FxTuning>,
    gangers: GangerSpriteWorld,
    positions: Query<&Position>,
    allocator: FctSlotAllocator,
    mut shots: MessageReader<Played<ShotFired>>,
) {
    let draw_scale = *tuning.projectile_draw_scale;
    let velocity = tuning.projectile_velocity;
    for msg in shots.read() {
        let muzzle_world = sim_pos_to_world(msg.muzzle);
        let target_world = ganger_hit_world(msg.kind, &gangers)
            .unwrap_or_else(|| cell_to_world(msg.impact_cell, msg.impact_level));

        let pops = classify_report(msg.report.as_ref());
        let anchor = anchor_cell(msg, &positions);

        let dir_index = nearest_direction_index(msg.trajectory.vec());
        let tile = sprites
            .fx_for(msg.damage)
            .directions
            .get(dir_index)
            .copied();
        let sprite = tile.and_then(|t| sprites.tile(t, Color::WHITE, draw_scale));
        let Some(sprite) = sprite else {
            spawn_pops_at_anchor(&mut commands, &allocator, &pops, anchor, &tuning);
            continue;
        };
        let launch_delay = std::time::Duration::ZERO;
        let travel = ProjectileTravel::new(
            ProjectileFlight::new(muzzle_world, target_world, velocity, launch_delay),
            ImpactPayload::new(msg.damage, pops, anchor, msg.shooter, msg.report.clone()),
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
    let at = CellLevel::new(cell, level);
    let base = *allocator.next_slot(at);
    for (offset, pop) in pops.iter().enumerate() {
        spawn_floating_text(
            commands,
            pop.label(),
            FctSlot::new(at, FctStackIndex::new(base + offset)),
            FctDrift::from_tuning(tuning),
        );
    }
}

fn ganger_hit_world(kind: ShotKind, gangers: &GangerSpriteWorld) -> Option<Vec3> {
    let ShotKind::Ganger(sim_entity) = kind else {
        return None;
    };
    gangers.position_of(sim_entity)
}
