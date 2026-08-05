mod appearance;
mod death;
mod frame;
mod roles;
mod spawn_move;
mod sprite_map;
mod tint;
mod tween;
mod visibility;

#[cfg(test)]
mod test;

pub use appearance::resolve_ganger_appearance;
pub use death::{
    despawn_killed_ganger_on_impact, despawn_removed_ganger_sprites, update_ganger_life_state,
};
pub use frame::{FacingFrame, facing_frame};
pub use roles::CharacterRoles;
pub(crate) use roles::register_character_roles_hot_ron;
pub use spawn_move::{move_ganger_sprites, spawn_ganger_sprites};
pub use sprite_map::{GangerSprite, GangerSpriteWorld, GangerSprites};
pub use tween::{SpriteTween, advance_sprite_tweens};
pub use visibility::{GangerVisibilityFacts, resolve_ganger_visibility};
