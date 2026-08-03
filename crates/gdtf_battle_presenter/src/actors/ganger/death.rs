use bevy::{ecs::lifecycle::RemovedComponents, prelude::*};
use gdtf_battle_sim::{
    prelude::{LifeState, Position},
    resolve_and_apply::HitReport,
    shot_fired::ShotFired,
};

use super::sprite_map::GangerSprites;
use crate::{
    ShotImpactResolved,
    playback::{DrawnLife, Played},
};

pub fn update_ganger_life_state(
    mut commands: Commands,
    mut sprites: ResMut<GangerSprites>,
    changed: Query<(Entity, &DrawnLife), Changed<DrawnLife>>,
    mut shots: MessageReader<Played<ShotFired>>,
) {
    let shot_killed: Vec<Entity> = shots
        .read()
        .filter_map(|played| shot_kill_victim(played))
        .collect();
    for (entity, drawn) in &changed {
        let life = &**drawn;
        if !matches!(life, LifeState::Dead) {
            continue;
        }
        if shot_killed.contains(&entity) {
            continue;
        }
        if let Some(presenter) = sprites.remove(entity) {
            commands.entity(presenter).despawn();
        }
    }
}

fn shot_kill_victim(shot: &ShotFired) -> Option<Entity> {
    report_kill_victim(shot.report.as_ref())
}

fn report_kill_victim(report: Option<&HitReport>) -> Option<Entity> {
    let report = report?;
    let gdtf_battle_sim::resolve_and_apply::HitVerdict::Ganger(verdict) = &report.verdict else {
        return None;
    };
    (verdict.applied.life_after == LifeState::Dead).then_some(verdict.target)
}

pub fn despawn_killed_ganger_on_impact(
    mut commands: Commands,
    mut sprites: ResMut<GangerSprites>,
    mut impacts: MessageReader<ShotImpactResolved>,
) {
    for impact in impacts.read() {
        let Some(victim) = report_kill_victim(impact.report.as_ref()) else {
            continue;
        };
        if let Some(presenter) = sprites.remove(victim) {
            commands.entity(presenter).despawn();
        }
    }
}

pub fn despawn_removed_ganger_sprites(
    mut commands: Commands,
    mut sprites: ResMut<GangerSprites>,
    mut removed: RemovedComponents<Position>,
) {
    for entity in removed.read() {
        if let Some(presenter) = sprites.remove(entity) {
            commands.entity(presenter).despawn();
        }
    }
}
