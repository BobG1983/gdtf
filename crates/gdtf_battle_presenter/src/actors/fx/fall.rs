//! Fall impact flash and floating combat text.

use bevy::prelude::*;
use gdtf_battle_sim::{falls::FallOccurred, prelude::Position};

use super::{
    fct::{
        CombatText, FctEmphasis, FctSlotAllocator, FctValence, spawn_floating_text, valence_color,
    },
    readers::spawn_flash,
    roles::EffectRoles,
};
use crate::{FxTuning, TopDownAtlases, cell_to_world, fx::readers::fx_sprite, playback::Played};

#[must_use]
fn fall_tint(storeys: u8) -> Color {
    let alpha = (f32::from(storeys) / 3.0).clamp(0.35, 1.0);
    Color::srgba(0.95, 0.55, 0.10, alpha)
}

/// Flash and "Fell" pop when a ganger falls.
pub fn read_fall_occurred(
    mut commands: Commands,
    atlases: Res<TopDownAtlases>,
    roles: Res<EffectRoles>,
    tuning: Res<FxTuning>,
    mut falls: MessageReader<Played<FallOccurred>>,
    positions: Query<&Position>,
    allocator: FctSlotAllocator,
) {
    for msg in falls.read() {
        let Ok(pos) = positions.get(msg.ganger) else {
            continue;
        };
        let at = **pos;
        let (cell, level) = at.split();
        let world = cell_to_world(cell, level);

        let storeys_raw = *msg.storeys;
        let tint = fall_tint(storeys_raw);
        if let Some(sprite) = fx_sprite(roles.fall_impact, tint, &atlases) {
            spawn_flash(&mut commands, sprite, world);
        }

        let slot = allocator.next_slot(at);
        spawn_floating_text(
            &mut commands,
            CombatText::new("Fell"),
            valence_color(FctValence::Neutral),
            FctEmphasis::Normal,
            cell,
            level,
            slot,
            tuning.fct_ttl_seconds,
            tuning.fct_rise_rate,
        );
    }
}
