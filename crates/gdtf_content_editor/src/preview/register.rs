//! The preview-machinery registrar — [`register_preview`], the state-scoped render-target /
//! redraw / view-apply system registrations (extracted so `mod.rs` stays fn-free).

use bevy::prelude::*;

use crate::{
    EditorState,
    preview::{
        target::{apply_preview_view, despawn_preview_target, spawn_preview_target},
        tiles::redraw_preview_tiles,
    },
};

/// Register the prefab preview render machinery (GTW-515 C4.3) onto the editor app.
///
/// - `OnEnter(Editing)` → [`spawn_preview_target`] (the offscreen image + egui registration + the
///   dedicated preview camera on the isolated render layer).
/// - `OnExit(Editing)` → [`despawn_preview_target`] (despawn the camera + tiles, remove the target).
/// - `Update` (in `Editing`) → [`redraw_preview_tiles`] (change-driven tile + hover-ghost redraw)
///   and [`apply_preview_view`] (the once-per-frame, set-to-target zoom/pan apply — bevy-traps #8
///   fact (b): OUTSIDE the egui closure so it never double-applies under multipass).
pub(crate) fn register_preview(app: &mut App) {
    app.add_systems(OnEnter(EditorState::Editing), spawn_preview_target);
    app.add_systems(OnExit(EditorState::Editing), despawn_preview_target);
    app.add_systems(
        Update,
        (redraw_preview_tiles, apply_preview_view).run_if(in_state(EditorState::Editing)),
    );
}
