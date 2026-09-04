//! Melee weapon authoring mode — draft and save.
mod draft;
mod save;

pub use draft::{MeleeWeaponDraft, structural_swing_mode};
pub use save::{draft_to_melee_weapon_spec, melee_weapon_file_name, melee_weapon_save_path_in};
#[cfg(feature = "mcp")]
pub use save::{write_melee_weapon, write_melee_weapon_in};

#[cfg(test)]
mod tests;
