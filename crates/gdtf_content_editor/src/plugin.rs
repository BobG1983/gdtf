//! Map editor Bevy plugin.

use bevy::prelude::*;
use bevy_egui::{EguiPrimaryContextPass, input::egui_wants_any_keyboard_input};
use gdtf_battle_presenter::{ContextDepth, IsolateView, ViewMode};
use gdtf_state_scoped::StateScopedResourceAppExt as _;

use crate::{
    EditorState,
    armor_form::ArmorDraft,
    attachment_form::AttachmentDraft,
    camera::{disable_egui_auto_context, spawn_editor_camera},
    canvas::{CanvasZoom, CurrentEditLevel},
    editor_map::EditorMap,
    egui_shell::{editor_egui_ui, level_nav_hotkeys, view_mode_hotkey},
    gang_form::GangDraft,
    hovered_cell::HoveredCell,
    injury_form::{InjuryDraft, WeightingDraft},
    load::register_load,
    melee_weapon_form::MeleeWeaponDraft,
    mode::{EditorMode, mode_hotkeys},
    preview::{register_preview, view::PreviewPan},
    right_panel::seed_default_theme,
    session::MapEditorSession,
    sprite_form::SpriteDraft,
    terrain_form::TerrainDraft,
    theme_form::ThemeDraft,
    validate::register_validation,
    weapon_form::WeaponDraft,
};

/// Registers editor states, scoped resources, and UI systems.
pub struct MapEditorPlugin;

impl Plugin for MapEditorPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<EditorState>();
        register_load(app);
        register_validation(app);

        app.add_systems(Startup, disable_egui_auto_context);

        app.add_systems(OnEnter(EditorState::Editing), spawn_editor_camera);

        app.init_state_scoped_resource(EditorState::Editing, EditorMode::default);
        app.init_state_scoped_resource(EditorState::Editing, MapEditorSession::default);
        app.init_state_scoped_resource(EditorState::Editing, EditorMap::new);
        app.init_state_scoped_resource(EditorState::Editing, CurrentEditLevel::ground);
        app.init_state_scoped_resource(EditorState::Editing, CanvasZoom::identity);
        app.init_state_scoped_resource(EditorState::Editing, TerrainDraft::default);
        app.init_state_scoped_resource(EditorState::Editing, ThemeDraft::default);
        app.init_state_scoped_resource(EditorState::Editing, GangDraft::default);
        app.init_state_scoped_resource(EditorState::Editing, ArmorDraft::default);
        app.init_state_scoped_resource(EditorState::Editing, InjuryDraft::default);
        app.init_state_scoped_resource(EditorState::Editing, WeightingDraft::default);
        app.init_state_scoped_resource(EditorState::Editing, SpriteDraft::default);
        app.init_state_scoped_resource(EditorState::Editing, AttachmentDraft::default);
        app.init_state_scoped_resource(EditorState::Editing, WeaponDraft::default);
        app.init_state_scoped_resource(EditorState::Editing, MeleeWeaponDraft::default);
        app.init_state_scoped_resource(EditorState::Editing, HoveredCell::new);
        app.init_state_scoped_resource(EditorState::Editing, PreviewPan::origin);
        app.init_state_scoped_resource(EditorState::Editing, ViewMode::default);
        app.init_state_scoped_resource(EditorState::Editing, || {
            IsolateView::On(ContextDepth::new(1))
        });

        register_preview(app);

        app.add_systems(
            EguiPrimaryContextPass,
            editor_egui_ui.run_if(in_state(EditorState::Editing)),
        );

        app.add_systems(
            Update,
            seed_default_theme.run_if(in_state(EditorState::Editing)),
        );

        app.add_systems(
            Update,
            (mode_hotkeys, level_nav_hotkeys, view_mode_hotkey)
                .run_if(in_state(EditorState::Editing))
                .run_if(not(egui_wants_any_keyboard_input)),
        );
    }
}
