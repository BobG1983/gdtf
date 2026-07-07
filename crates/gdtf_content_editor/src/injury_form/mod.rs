//! The INJURY authoring mode's MODEL half (GTW-654) — the Workbench forms that edit
//! an injury def (`*.injury.ron`: severity / category / texts / the closed-palette
//! effects list) AND a per-category weighting table (`*.weighting.ron`), saving both
//! where the bespoke GTW-437 injuries folder loader reads.
//!
//! Wiring-only module (module-layout rule 2). The working models (the two
//! state-scoped resources — the [`InjuryDraft`] def form and the [`WeightingDraft`]
//! table form) live in [`draft`] / [`weighting`]; the loader-schema projections +
//! the one-owner save paths live in [`save`]. The egui FORMS that draw over these
//! models are the shell's `egui_shell::injury_form_ui` sibling (the GTW-636 gang
//! form split: model here, draw there). Follows the Gang / Armor modes at parity of
//! pattern: load-any, create, edit fields (incl. effects-list add/remove), save,
//! load-back.

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
