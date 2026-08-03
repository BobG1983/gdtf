mod auto_select;
mod decision;
mod fire_target;
mod highlight;
mod order;
mod path_preview;
/// Reachable-range overlay populate — debug builds only.
#[cfg(debug_assertions)]
mod reachable;
mod resources;
mod systems;

pub use auto_select::{auto_select_first_player_ganger, clear_downed_selection};
pub use decision::{
    LeftClickOutcome, LeftClickReads, PinOutcome, TurnReads, apply_left_click, apply_pin,
    decide_left_click, decide_pin, decide_turn,
};
pub use fire_target::{FireTargetReads, populate_fire_target};
pub use highlight::update_selection_highlight;
pub use order::{CellOrderKey, CycleDirection, cell_order_key, cycle_player_selection};
pub use path_preview::{
    PathPreviewTarget, PreviewGrids, populate_path_preview, reset_move_target_on_fire_mode_change,
};
// Reachable overlay populate is debug-only.
#[cfg(debug_assertions)]
pub use reachable::{ReachableGrids, populate_reachable_overlay};
pub use resources::{SelectedShooter, SelectionHighlight};
pub use systems::{left_click_act, right_click_turn_to_face};
