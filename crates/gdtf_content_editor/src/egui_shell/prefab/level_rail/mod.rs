//! The PREFAB-mode **level rail** (GTW-595) — the per-storey occupancy thumbnail
//! scrub-strip in the right panel that replaced the blind `Level n / m` paging
//! (GTW-593 Option 3).
//!
//! Wiring-only module. The pure CPU occupancy sweep + the change-key live in
//! [`occupancy`]; the change-keyed thumbnail cache + the rail's `Local` view state live
//! in [`cache`]; the wheel-scrub fold lives in [`scrub`]; the egui draw + row
//! interaction live in [`rail_ui`]. EDITOR-ONLY (GTW-595 C3): the rail reads the
//! authoring-truth [`EditorMap`](crate::editor_map::EditorMap) and lives entirely in
//! this crate — nothing here is reachable from the battlescape presenter, so authored
//! occupancy can never leak through the fog canon.

mod cache;
mod occupancy;
mod rail_ui;
mod scrub;
#[cfg(test)]
mod test;

pub(crate) use cache::RailUiState;
pub(crate) use rail_ui::{RailCtx, level_rail};
