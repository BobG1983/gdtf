//! Editor writes that change a draft, a mode tab, or a file on disk.

mod blank;
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

pub(in crate::net_qa) use blank::EditorNew;
pub(in crate::net_qa) use list_op::EditorListOp;
pub(in crate::net_qa) use load::{
    EditorLoadArmor, EditorLoadAttachment, EditorLoadField, EditorLoadGang, EditorLoadInjury,
    EditorLoadMeleeWeapon, EditorLoadSprite, EditorLoadTheme, EditorLoadWeapon,
};
pub(in crate::net_qa) use paint::EditorPaint;
pub(in crate::net_qa) use save::EditorSave;
pub(in crate::net_qa) use save_weighting::EditorSaveWeighting;
pub(in crate::net_qa) use select_facing::EditorSelectFacing;
pub(in crate::net_qa) use select_injury_tab::EditorSelectInjuryTab;
pub(in crate::net_qa) use select_theme::EditorSelectTheme;
pub(in crate::net_qa) use select_tile::EditorSelectTile;
pub(in crate::net_qa) use select_weighting_table::EditorSelectWeightingTable;
pub(in crate::net_qa) use set_default_floor::EditorSetDefaultFloor;
pub(in crate::net_qa) use set_field::EditorSetField;
pub(in crate::net_qa) use set_grid_size::EditorSetGridSize;
pub(in crate::net_qa) use set_level::EditorSetLevel;
pub(in crate::net_qa) use set_mode::EditorSetMode;
pub(in crate::net_qa) use toggle_terrain::EditorToggleTerrain;
