use bevy::prelude::*;
use gdtf_battle_sim::{
    ganger::{Aiming, Facing},
    prelude::{Faction, LifeState, Stance},
};

use super::{
    frame::atlas_index,
    roles::CharacterRoles,
    sprite_map::{GangerSprite, GangerSprites},
    tint::stance_aiming_tint,
};
use crate::playback::{DrawnLife, DrawnPose};

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct GangerAppearance {
            pub(super) atlas_index: usize,
            pub(super) tint:        Color,
}

#[must_use]
pub(super) fn ganger_sprite_appearance(
    faction: Faction,
    facing: Facing,
    stance: Stance,
    aiming: Aiming,
    life: LifeState,
    suppressed: bool,
    roles: &CharacterRoles,
) -> GangerAppearance {
    GangerAppearance {
        atlas_index: atlas_index(roles, faction, facing),
        tint:        stance_aiming_tint(faction, life, stance, aiming, suppressed),
    }
}

type AppearanceData = (
    Entity,
    &'static Faction,
    &'static DrawnPose,
    &'static DrawnLife,
);

type AppearanceChanged = Or<(Changed<DrawnPose>, Changed<DrawnLife>)>;

pub fn resolve_ganger_appearance(
    sprites: Res<GangerSprites>,
    roles: Res<CharacterRoles>,
    changed: Query<AppearanceData, AppearanceChanged>,
    all: Query<AppearanceData>,
    mut presenters: Query<&mut Sprite, With<GangerSprite>>,
) {
    if roles.is_changed() {
        for data in &all {
            stamp_appearance(&sprites, &roles, &mut presenters, data);
        }
        return;
    }
    for data in &changed {
        stamp_appearance(&sprites, &roles, &mut presenters, data);
    }
}

fn stamp_appearance(
    sprites: &GangerSprites,
    roles: &CharacterRoles,
    presenters: &mut Query<&mut Sprite, With<GangerSprite>>,
    data: (Entity, &Faction, &DrawnPose, &DrawnLife),
) {
    let (entity, faction, pose, life) = data;
    let life = **life;
    if matches!(life, LifeState::Dead) {
        return;
    }
    let Some(presenter) = sprites.sprite_for(entity) else {
        return;
    };
    let Ok(mut sprite) = presenters.get_mut(presenter) else {
        return;
    };
    let appearance = ganger_sprite_appearance(
        *faction,
        pose.facing(),
        pose.stance(),
        pose.aiming(),
        life,
        pose.suppressed(),
        roles,
    );
    if let Some(atlas) = sprite.texture_atlas.as_mut() {
        atlas.index = appearance.atlas_index;
    }
    sprite.color = appearance.tint;
}
