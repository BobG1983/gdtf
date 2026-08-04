use bevy::prelude::*;
use gdtf_battle_sim::{
    ganger::Aiming,
    prelude::{Faction, LifeState, Stance, StanceKind},
};

#[must_use]
pub(super) fn faction_tint(faction: Faction) -> Color {
    match *faction {
        1 => Color::srgb(1.0, 0.55, 0.5),
        _ => Color::srgb(0.55, 0.7, 1.0),
    }
}

#[must_use]
const fn downed_tint() -> Color {
    Color::srgb(0.4, 0.4, 0.45)
}

#[must_use]
pub(super) fn ganger_tint(faction: Faction, life: LifeState) -> Color {
    match life {
        LifeState::Downed => downed_tint(),
        LifeState::Alive | LifeState::Dead => faction_tint(faction),
    }
}

#[must_use]
pub(super) fn stance_aiming_tint(
    faction: Faction,
    life: LifeState,
    stance: Stance,
    aiming: Aiming,
    suppressed: bool,
) -> Color {
    let base = ganger_tint(faction, life);
    if matches!(life, LifeState::Downed) {
        return base;
    }
    let stance_scale = match *stance {
        StanceKind::Prone => 0.7,
        StanceKind::Standing | StanceKind::Crouching => 1.0,
    };
    let aim_scale = if *aiming { 1.2 } else { 1.0 };
    let factor = stance_scale * aim_scale;
    let linear = base.to_linear();
    let value_shifted = Color::linear_rgba(
        linear.red * factor,
        linear.green * factor,
        linear.blue * factor,
        linear.alpha,
    );
    if suppressed {
        suppressed_tint(value_shifted)
    } else {
        value_shifted
    }
}

#[must_use]
fn suppressed_tint(tint: Color) -> Color {
    const SUPPRESSED_DESATURATION: f32 = 0.6;
    const SUPPRESSED_DARKEN: f32 = 0.75;

    let linear = tint.to_linear();
    let grey = (linear.red + linear.green + linear.blue) / 3.0;
    let mix = |channel: f32| {
        channel.mul_add(
            1.0 - SUPPRESSED_DESATURATION,
            grey * SUPPRESSED_DESATURATION,
        ) * SUPPRESSED_DARKEN
    };
    Color::linear_rgba(
        mix(linear.red),
        mix(linear.green),
        mix(linear.blue),
        linear.alpha,
    )
}
