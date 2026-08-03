use bevy::prelude::*;

use crate::{
    EditorState,
    preview::{
        overlay::{create_preview_overlay_images, remove_preview_overlay_images},
        target::{apply_preview_view, despawn_preview_target, spawn_preview_target},
        tiles::redraw_preview_tiles,
    },
};

pub(crate) fn register_preview(app: &mut App) {
    app.add_systems(
        OnEnter(EditorState::Editing),
        (spawn_preview_target, create_preview_overlay_images),
    );
    app.add_systems(
        OnExit(EditorState::Editing),
        (despawn_preview_target, remove_preview_overlay_images),
    );
    app.add_systems(
        Update,
        (redraw_preview_tiles, apply_preview_view).run_if(in_state(EditorState::Editing)),
    );
}
