use bevy::prelude::*;
use gdtf_battle_sim::{acts::MeleeResolved, weapon::DamageType};

use super::{readers::spawn_flash, roles::EffectRoles};
use crate::{TopDownAtlases, cell_to_world, fx::readers::fx_sprite, playback::Played};

const fn strike_tint(damage: DamageType) -> Color {
    match damage {
        DamageType::Kinetic | DamageType::Blast => Color::srgb(1.0, 0.75, 0.35),
        DamageType::Las | DamageType::Shock => Color::srgb(0.55, 0.85, 1.0),
        DamageType::Chem => Color::srgb(0.55, 1.0, 0.45),
        DamageType::Plasma | DamageType::Rend => Color::srgb(0.85, 0.5, 1.0),
    }
}

pub fn read_melee_resolved(
    mut commands: Commands,
    atlases: Res<TopDownAtlases>,
    roles: Res<EffectRoles>,
    mut resolved: MessageReader<Played<MeleeResolved>>,
) {
    for msg in resolved.read() {
        let (cell, level) = msg.at.split();
        let Some(sprite) = fx_sprite(roles.melee_strike, strike_tint(msg.damage), &atlases) else {
            continue;
        };
        spawn_flash(&mut commands, sprite, cell_to_world(cell, level));
    }
}
