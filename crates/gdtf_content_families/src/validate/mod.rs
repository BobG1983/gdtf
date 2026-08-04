//! Authoring-time reference checks over content families.
//! Dangling keys fail in the editor, not on the next game launch.
mod attachments;
mod gangs;
mod injuries;
mod sprites;
mod terrain;

pub use attachments::check_weapon_attachment_refs;
pub use gangs::check_gang_equipment_refs;
pub use injuries::check_injury_weighting_refs;
pub use sprites::check_terrain_graphic_refs;
pub use terrain::{check_emplacement_weapon_refs, check_theme_terrain_refs};
