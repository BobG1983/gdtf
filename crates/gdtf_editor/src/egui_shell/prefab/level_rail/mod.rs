//! authoring-truth [`EditorMap`](crate::editor_map::EditorMap) and lives entirely in
//! this crate — nothing here is reachable from the battlescape presenter, so authored
mod cache;
mod occupancy;
mod rail_ui;
mod scrub;
#[cfg(test)]
mod test;

pub(crate) use cache::RailUiState;
pub(crate) use rail_ui::{RailCtx, level_rail};
