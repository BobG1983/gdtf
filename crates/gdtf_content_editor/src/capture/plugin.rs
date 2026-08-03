use std::env;

use bevy::{prelude::*, state::state::OnEnter};
use gdtf_screenshot::{
    CapturePath, CaptureProgress, PollCap, SettleFrames, parse_shot_path, poll_then_exit,
    reset_progress, settle_then_capture,
};

use super::{
    drive::{
        drive_capture_grid_size, drive_capture_paint_and_hover, drive_capture_selection,
        force_capture_attachment, force_capture_melee_weapon, force_capture_mode,
        force_capture_terrain_kind, force_capture_view, force_capture_weapon, force_capture_zoom,
    },
    forced::{
        ForcedAttachment, ForcedMeleeWeapon, ForcedMode, ForcedTerrainKind, ForcedView,
        ForcedWeapon, ForcedZoom,
    },
};
use crate::EditorState;

const SHOT_ENV_VAR: &str = "GDTF_EDITOR_SHOT";

const MODE_ENV_VAR: &str = "GDTF_EDITOR_MODE";

const TERRAIN_KIND_ENV_VAR: &str = "GDTF_EDITOR_TERRAIN_KIND";

const ATTACHMENT_ENV_VAR: &str = "GDTF_EDITOR_ATTACHMENT";

const WEAPON_ENV_VAR: &str = "GDTF_EDITOR_WEAPON";

const MELEE_WEAPON_ENV_VAR: &str = "GDTF_EDITOR_MELEE_WEAPON";

const ZOOM_ENV_VAR: &str = "GDTF_EDITOR_ZOOM";

const VIEW_ENV_VAR: &str = "GDTF_EDITOR_VIEW";

pub struct EditorCapturePlugin {
            path:              Option<CapturePath>,
            forced_mode:       Option<ForcedMode>,
            forced_kind:       Option<ForcedTerrainKind>,
            forced_attachment: Option<ForcedAttachment>,
            forced_weapon:     Option<ForcedWeapon>,
                forced_melee:      Option<ForcedMeleeWeapon>,
            forced_zoom:       Option<ForcedZoom>,
                forced_view:       Option<ForcedView>,
}

impl EditorCapturePlugin {
                                #[must_use]
    pub fn from_env() -> Self {
        Self {
            path:              parse_shot_path(env::var(SHOT_ENV_VAR).ok().as_deref()),
            forced_mode:       env::var(MODE_ENV_VAR)
                .ok()
                .as_deref()
                .and_then(ForcedMode::from_env_value),
            forced_kind:       env::var(TERRAIN_KIND_ENV_VAR)
                .ok()
                .as_deref()
                .and_then(ForcedTerrainKind::from_env_value),
            forced_attachment: env::var(ATTACHMENT_ENV_VAR)
                .ok()
                .as_deref()
                .and_then(ForcedAttachment::from_env_value),
            forced_weapon:     env::var(WEAPON_ENV_VAR)
                .ok()
                .as_deref()
                .and_then(ForcedWeapon::from_env_value),
            forced_melee:      env::var(MELEE_WEAPON_ENV_VAR)
                .ok()
                .as_deref()
                .and_then(ForcedMeleeWeapon::from_env_value),
            forced_zoom:       env::var(ZOOM_ENV_VAR)
                .ok()
                .as_deref()
                .and_then(ForcedZoom::from_env_value),
            forced_view:       env::var(VIEW_ENV_VAR)
                .ok()
                .as_deref()
                .and_then(ForcedView::from_env_value),
        }
    }
}

impl Plugin for EditorCapturePlugin {
    fn build(&self, app: &mut App) {
        let Some(path) = self.path.clone() else {
            return;
        };
        app.insert_resource(path)
            .insert_resource(SettleFrames::DEFAULT_EGUI)
            .insert_resource(PollCap::DEFAULT)
            .init_resource::<CaptureProgress>()
            .add_systems(OnEnter(EditorState::Editing), reset_progress);
        if let Some(forced) = self.forced_mode {
            app.insert_resource(forced);
        }
        if let Some(forced) = self.forced_kind {
            app.insert_resource(forced);
        }
        if let Some(forced) = self.forced_attachment.clone() {
            app.insert_resource(forced);
        }
        if let Some(forced) = self.forced_weapon.clone() {
            app.insert_resource(forced);
        }
        if let Some(forced) = self.forced_melee.clone() {
            app.insert_resource(forced);
        }
        if let Some(forced) = self.forced_zoom {
            app.insert_resource(forced);
        }
        if let Some(forced) = self.forced_view {
            app.insert_resource(forced);
        }
        app.add_systems(
            Update,
            (
                force_capture_mode,
                force_capture_terrain_kind,
                force_capture_attachment,
                force_capture_weapon,
                force_capture_melee_weapon,
                force_capture_zoom,
                force_capture_view,
                drive_capture_grid_size,
                drive_capture_selection,
                drive_capture_paint_and_hover,
                settle_then_capture,
                poll_then_exit,
            )
                .chain()
                .run_if(in_state(EditorState::Editing)),
        );
    }
}
