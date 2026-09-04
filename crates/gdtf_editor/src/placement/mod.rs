mod classification;
mod rules;
mod verdict;

#[cfg(test)]
mod tests;

pub use classification::{EditorTileClass, classify, names_a_ladder};
pub use rules::{apply_placement, evaluate_placement};
pub use verdict::{IllegalReason, PlacementVerdict, ProposedPlacement};
