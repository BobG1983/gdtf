//! Authoring-time reference checks over content families.
//! Dangling keys fail in the editor, not on the next game launch.
mod attachments;
mod gangs;
mod injuries;
mod prefabs;
mod sprites;
mod terrain;
mod terrain_views;

pub use attachments::check_weapon_attachment_refs;
pub use gangs::check_gang_equipment_refs;
pub use injuries::check_injury_weighting_refs;
pub use prefabs::check_prefab_refs;
pub use sprites::check_terrain_view_sprite_refs;
pub use terrain::{
    check_emplacement_weapon_refs, check_terrain_leaves_behind_refs, check_theme_terrain_refs,
};
pub use terrain_views::check_terrain_view_coverage;
