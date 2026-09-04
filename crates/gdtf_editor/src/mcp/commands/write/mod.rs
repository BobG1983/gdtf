//! Editor writes that change a draft, a mode tab, or a file on disk.

mod blank;
mod delete_record;
mod form_fault;
mod list_op;
mod load;
mod paint;
mod save;
mod save_weighting;
mod select_facing;
mod select_injury_tab;
mod select_theme;
mod select_tile;
mod select_weighting_table;
mod set_default_floor;
mod set_field;
mod set_grid_size;
mod set_level;
mod set_mode;
mod toggle_terrain;
mod weighting_rows;

pub(in crate::mcp) use blank::EditorNew;
pub(in crate::mcp) use delete_record::EditorDeleteRecord;
pub(in crate::mcp) use list_op::EditorListOp;
pub(in crate::mcp) use load::{
    EditorLoadArmor, EditorLoadAttachment, EditorLoadField, EditorLoadGang, EditorLoadInjury,
    EditorLoadMeleeWeapon, EditorLoadPrefab, EditorLoadSprite, EditorLoadTerrain, EditorLoadTheme,
    EditorLoadWeapon,
};
pub(in crate::mcp) use paint::EditorPaint;
pub(in crate::mcp) use save::EditorSave;
pub(in crate::mcp) use save_weighting::EditorSaveWeighting;
pub(in crate::mcp) use select_facing::EditorSelectFacing;
pub(in crate::mcp) use select_injury_tab::EditorSelectInjuryTab;
pub(in crate::mcp) use select_theme::EditorSelectTheme;
pub(in crate::mcp) use select_tile::EditorSelectTile;
pub(in crate::mcp) use select_weighting_table::EditorSelectWeightingTable;
pub(in crate::mcp) use set_default_floor::EditorSetDefaultFloor;
pub(in crate::mcp) use set_field::EditorSetField;
pub(in crate::mcp) use set_grid_size::EditorSetGridSize;
pub(in crate::mcp) use set_level::EditorSetLevel;
pub(in crate::mcp) use set_mode::EditorSetMode;
pub(in crate::mcp) use toggle_terrain::EditorToggleTerrain;
