//! Hot-loaded effect sheet tile indices and damage-type FX blocks.

use bevy::{math::Vec3, prelude::*};
use cobalt_ron_assets::HotRonAppExt;
use gdtf_battle_sim::weapon::DamageType;
use serde::Deserialize;

use crate::TileIndex;

/// Number of compass directions on the effects sheet.
pub const DIRECTION_COUNT: usize = 8;

/// Frames in an impact animation strip.
pub const IMPACT_FRAME_COUNT: usize = 3;

/// Unit vectors for the eight compass directions used by projectile frames.
pub const COMPASS_DIRECTIONS: [Vec2; DIRECTION_COUNT] = {
    const D: f32 = 0.707_106_77;
    [
        Vec2::new(1.0, 0.0),
        Vec2::new(D, -D),
        Vec2::new(0.0, -1.0),
        Vec2::new(-D, -D),
        Vec2::new(0.0, 1.0),
        Vec2::new(-D, D),
        Vec2::new(-1.0, 0.0),
        Vec2::new(D, D),
    ]
};

/// Directional projectile frames and impact strip for one damage family.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct DamageTypeFx {
    /// One tile per compass direction.
    pub directions: [TileIndex; DIRECTION_COUNT],
    /// Impact animation frames.
    pub impact:     [TileIndex; IMPACT_FRAME_COUNT],
}

/// Authored tile indices for all effect families.
#[derive(Resource, Debug, Clone, PartialEq, Eq, Deserialize, TypePath)]
pub struct EffectRoles {
    /// Bleed tick flash.
    pub bleed:           TileIndex,
    /// Armor break flash.
    pub armor_break:     TileIndex,
    /// Cover destroyed flash.
    pub cover_destroyed: TileIndex,
    /// Melee strike flash.
    pub melee_strike:    TileIndex,
    /// Fall impact flash.
    pub fall_impact:     TileIndex,
    /// Kinetic / blast projectile family.
    pub orange:          DamageTypeFx,
    /// Las / shock projectile family.
    pub blue:            DamageTypeFx,
    /// Chem projectile family.
    pub green:           DamageTypeFx,
    /// Plasma / rend projectile family.
    pub purple:          DamageTypeFx,
}

impl EffectRoles {
    /// FX block for a damage type.
    #[must_use]
    pub const fn fx_for(&self, damage: DamageType) -> &DamageTypeFx {
        match damage {
            DamageType::Kinetic | DamageType::Blast => &self.orange,
            DamageType::Las | DamageType::Shock => &self.blue,
            DamageType::Chem => &self.green,
            DamageType::Plasma | DamageType::Rend => &self.purple,
        }
    }

    /// Fallback when a specific family is missing.
    #[must_use]
    pub const fn fallback(&self) -> &DamageTypeFx {
        &self.orange
    }
}

/// Index of the compass direction closest to `trajectory`.
#[must_use]
pub fn nearest_direction_index(trajectory: Vec3) -> usize {
    let heading = trajectory.truncate();
    if heading.length_squared() <= f32::EPSILON {
        return 0;
    }
    let heading = heading.normalize_or_zero();
    let mut best_index = 0;
    let mut best_dot = f32::NEG_INFINITY;
    for (index, dir) in COMPASS_DIRECTIONS.iter().enumerate() {
        let dot = heading.dot(*dir);
        if dot > best_dot {
            best_dot = dot;
            best_index = index;
        }
    }
    best_index
}

const EFFECT_ROLES_RON_PATH: &str = "sprites/effect_roles.spritedef.ron";

pub(crate) fn register_effect_roles_hot_ron(app: &mut App) {
    app.init_hot_ron_resource::<EffectRoles>(EFFECT_ROLES_RON_PATH);
}
