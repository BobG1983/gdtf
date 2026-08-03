//! Content editor: map, terrain, theme, and equipment form tools.

mod app;
mod armor_form;
mod attachment_form;
mod camera;
mod canvas;
mod capture;
mod connector_pairing;
mod editor_map;
mod egui_shell;
mod gang_form;
mod hovered_cell;
mod injury_form;
mod load;
mod melee_weapon_form;
mod mode;
#[cfg(all(debug_assertions, feature = "net_qa"))]
mod net_qa;
mod placement;
mod plugin;
mod preview;
mod right_panel;
#[cfg(debug_assertions)]
mod save;
mod session;
mod sprite_form;
mod state;
mod terrain_form;
mod theme_form;
mod terrain_graphics;
mod theme_dir;
mod validate;
mod weapon_form;

pub use app::MapEditorApp;
pub use armor_form::{ArmorDraft, armor_file_name, armor_save_path_in, draft_to_spec};
#[cfg(debug_assertions)]
pub use armor_form::{write_armor, write_armor_in};
pub use attachment_form::{
    AttachmentDraft, attachment_file_name, attachment_save_path_in, draft_to_attachment_spec,
};
#[cfg(debug_assertions)]
pub use attachment_form::{write_attachment, write_attachment_in};
pub use canvas::{CanvasZoom, CurrentEditLevel, LevelStep};
pub use capture::EditorCapturePlugin;
pub use connector_pairing::{
    PairingOutcome, apply_placement_with_pairing, is_up_connector, resolve_down_counterpart,
};
pub use editor_map::EditorMap;
pub use egui_shell::prefab::size_fields::SizeFieldSpans;
pub use gang_form::{GangDraft, draft_to_roster, gang_file_name, gang_save_path_in};
#[cfg(debug_assertions)]
pub use gang_form::{write_gang, write_gang_in};
pub use hovered_cell::HoveredCell;
pub use injury_form::{
    InjuryDraft, WeightingDraft, draft_to_def, draft_to_weighting, injury_file_name,
    injury_save_path_in, weighting_file_name, weighting_save_path_in,
};
#[cfg(debug_assertions)]
pub use injury_form::{write_injury, write_injury_in, write_weighting, write_weighting_in};
pub use melee_weapon_form::{
    MeleeWeaponDraft, draft_to_melee_weapon_spec, melee_weapon_file_name, melee_weapon_save_path_in,
};
#[cfg(debug_assertions)]
pub use melee_weapon_form::{write_melee_weapon, write_melee_weapon_in};
pub use mode::EditorMode;
#[cfg(all(debug_assertions, feature = "net_qa"))]
pub use net_qa::{
    EDITOR_QA_SERVER_NAME, EditorNetQaSystems, EditorQaShotDir, EditorScreenshotPayload,
    EditorShotPollBudget, EditorShotSettle, EditorShotSource, NetQaEditorPlugin,
};
pub use placement::{
    EditorTileClass, IllegalReason, PlacementVerdict, ProposedPlacement, apply_placement, classify,
    evaluate_placement, names_a_ladder,
};
pub use plugin::MapEditorPlugin;
pub use preview::{target::PreviewTarget, view::PreviewPan};
pub use right_panel::GridSpanInput;
#[cfg(debug_assertions)]
pub use save::{
    SavePrefabError, editor_map_to_prefab, prefab_save_path, prefab_save_path_in, sanitize_name,
    serialize_prefab, write_prefab, write_prefab_in,
};
pub use session::MapEditorSession;
pub use sprite_form::{SpriteDraft, draft_to_sprite_def, sprite_file_name, sprite_save_path_in};
#[cfg(debug_assertions)]
pub use sprite_form::{write_sprite, write_sprite_in};
pub use state::EditorState;
pub use terrain_form::{
    ArmorInput, FootfallChoice, HpInput, SaveTerrainError, TerrainDraft, TerrainKindChoice,
    draft_to_terrain_def, offered_graphic_roles, serialize_terrain_def,
};
#[cfg(debug_assertions)]
pub use terrain_form::{write_terrain, write_terrain_in};
pub use theme_form::{
    SaveThemeError, ThemeDraft, draft_to_theme_def, floor_candidates, resolved_stats,
    serialize_theme_def, slab_floor_candidates, validate_for_save,
};
#[cfg(debug_assertions)]
pub use theme_form::{write_theme, write_theme_in};
pub use weapon_form::{WeaponDraft, draft_to_weapon_spec, weapon_file_name, weapon_save_path_in};
#[cfg(debug_assertions)]
pub use weapon_form::{write_weapon, write_weapon_in};
