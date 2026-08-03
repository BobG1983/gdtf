//! The GANG authoring mode's MODEL half (GTW-636) — the Workbench form that edits a
//! authored OUTSIDE the game binary — this mode replaces the retired in-game gang
mod draft;
mod save;

pub use draft::GangDraft;
pub use save::{draft_to_roster, gang_file_name, gang_save_path_in};
#[cfg(debug_assertions)]
pub use save::{write_gang, write_gang_in};

#[cfg(test)]
mod tests;
