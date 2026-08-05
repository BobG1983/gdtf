//! Effect-sheet tile lookup shared by the FX systems.

use bevy::{ecs::system::SystemParam, prelude::*};
use gdtf_battle_sim::weapon::DamageType;

use super::{
    readers::fx_sprite_scaled,
    roles::{DamageTypeFx, EffectRoles},
};
use crate::{TileIndex, TopDownAtlases};

/// The authored effect tiles and the sheet they are cut from.
#[derive(SystemParam)]
pub struct FxSprites<'w> {
    roles:   Res<'w, EffectRoles>,
    atlases: Res<'w, TopDownAtlases>,
}

impl FxSprites<'_> {
    /// Projectile and impact tiles for a damage family.
    #[must_use]
    pub fn fx_for(&self, damage: DamageType) -> &DamageTypeFx {
        self.roles.fx_for(damage)
    }

    /// Sprite for one effect-sheet tile, tinted and scaled.
    #[must_use]
    pub fn tile(&self, index: TileIndex, tint: Color, scale: f32) -> Option<Sprite> {
        fx_sprite_scaled(index, tint, scale, &self.atlases)
    }
}
