//! SAME check over the same family glue, and a dangling key authored in the
//! editor surfaces at authoring time, not on the next game launch).
//! of the authored content graph and appends a typed
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
