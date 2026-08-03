use bevy::{math::Vec3, prelude::*};
use gdtf_assets::HotRonAppExt;
use gdtf_battle_sim::weapon::DamageType;
use serde::Deserialize;

use crate::TileIndex;

pub const DIRECTION_COUNT: usize = 8;

pub const IMPACT_FRAME_COUNT: usize = 3;

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

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct DamageTypeFx {
                pub directions: [TileIndex; DIRECTION_COUNT],
                pub impact:     [TileIndex; IMPACT_FRAME_COUNT],
}

#[derive(Resource, Debug, Clone, PartialEq, Eq, Deserialize, TypePath)]
pub struct EffectRoles {
        pub bleed:           TileIndex,
        pub armor_break:     TileIndex,
        pub cover_destroyed: TileIndex,
                pub melee_strike:    TileIndex,
                pub fall_impact:     TileIndex,
        pub orange:          DamageTypeFx,
        pub blue:            DamageTypeFx,
        pub green:           DamageTypeFx,
        pub purple:          DamageTypeFx,
}

impl EffectRoles {
                                            #[must_use]
    pub const fn fx_for(&self, damage: DamageType) -> &DamageTypeFx {
        match damage {
            DamageType::Kinetic | DamageType::Blast => &self.orange,
            DamageType::Las | DamageType::Shock => &self.blue,
            DamageType::Chem => &self.green,
            DamageType::Plasma | DamageType::Rend => &self.purple,
        }
    }

                                #[must_use]
    pub const fn fallback(&self) -> &DamageTypeFx {
        &self.orange
    }
}

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
