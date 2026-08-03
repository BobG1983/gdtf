//! The INJURY authoring mode's MODEL half (GTW-654) — the Workbench forms that edit
mod draft;
mod save;
mod weighting;

pub(crate) use draft::DEFAULT_EFFECT;
pub use draft::InjuryDraft;
pub use save::{
    draft_to_def, draft_to_weighting, injury_file_name, injury_save_path_in, weighting_file_name,
    weighting_save_path_in,
};
#[cfg(debug_assertions)]
pub use save::{write_injury, write_injury_in, write_weighting, write_weighting_in};
pub use weighting::WeightingDraft;

#[cfg(test)]
mod tests;
