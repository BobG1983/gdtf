//! The env-gated capture PLUGIN — reads the `GDTF_EDITOR_*` env vars ONCE at
//! construction and, when enabled, registers the drive + the delegated
//! `gdtf_screenshot` capture chain.

use std::env;

use bevy::{prelude::*, state::state::OnEnter};
use gdtf_screenshot::{
    CapturePath, CaptureProgress, PollCap, SettleFrames, parse_shot_path, poll_then_exit,
    reset_progress, settle_then_capture,
};

use super::{
    drive::{
        drive_capture_grid_size, drive_capture_paint_and_hover, drive_capture_selection,
        force_capture_attachment, force_capture_mode, force_capture_terrain_kind,
        force_capture_view, force_capture_weapon, force_capture_zoom,
    },
    forced::{
        ForcedAttachment, ForcedMode, ForcedTerrainKind, ForcedView, ForcedWeapon, ForcedZoom,
    },
};
use crate::EditorState;

/// The env var that opts the capture affordance IN. Set it to an absolute PNG path; leave it unset
/// for a normal interactive launch.
const SHOT_ENV_VAR: &str = "GDTF_EDITOR_SHOT";

/// The env var that FORCES the [`crate::EditorMode`] before the capture (C2.4) — so the
/// Screenshot-QA can capture a SPECIFIC Workbench mode (`terrain` | `theme` | `prefab` |
/// `gang` | `armor` | `injury` | `sprite` | `attachment` | `weapon`, case-insensitive). Unset
/// (or an unrecognized value) keeps the default mode the editor
/// opened in. Honored only when the capture affordance itself is enabled (`GDTF_EDITOR_SHOT` set).
const MODE_ENV_VAR: &str = "GDTF_EDITOR_MODE";

/// The env var that pre-selects the TERRAIN form's kind segment before the capture (GTW-574 —
/// `wall` | `cover` | `slab` | `emplacement`, case-insensitive), so QA can capture a specific
/// kind's authoring surface (for `emplacement`, with the mounted-weapon dropdown POPULATED from
/// the live registry). Unset (or an unrecognized value) keeps the draft's default kind. Honored
/// only when the capture affordance itself is enabled.
const TERRAIN_KIND_ENV_VAR: &str = "GDTF_EDITOR_TERRAIN_KIND";

/// The env var that pre-loads a NAMED attachment item into the ATTACHMENT form before the
/// capture (GTW-669 — a registry key / file stem, e.g. `scoped_sight`), so QA can capture a
/// specific item's slot + effect rows (the GTW-574 terrain-kind precedent). Unset (or a key that
/// resolves nothing) keeps the mode's sorted-first autoload. Honored only when the capture
/// affordance itself is enabled.
const ATTACHMENT_ENV_VAR: &str = "GDTF_EDITOR_ATTACHMENT";

/// The env var that pre-loads a NAMED weapon into the WEAPON form before the capture
/// (GTW-670 — a registry key / file stem, e.g. `heavy_bolter`), so QA can capture a specific
/// weapon's full spec form (the GTW-669 `ForcedAttachment` precedent). Unset (or a key that
/// resolves nothing) keeps the mode's sorted-first autoload. Honored only when the capture
/// affordance itself is enabled.
const WEAPON_ENV_VAR: &str = "GDTF_EDITOR_WEAPON";

/// The env var that FORCES a non-`1.0` preview [`crate::canvas::CanvasZoom`] scale before the
/// capture (GTW-515 C4.11) — a float in `[0.25, 4.0]`, clamped on apply. Unset keeps the identity
/// `1.0` scale. Honored only when the capture affordance is enabled AND the mode is (forced to)
/// PREFAB.
const ZOOM_ENV_VAR: &str = "GDTF_EDITOR_ZOOM";

/// The env var that FORCES the prefab-viewport storey view before the capture — `full`
/// (GTW-532) selects the whole-stack view (lifting the GTW-594 Isolate default, which
/// would win over it); `isolate` (GTW-594) asserts the Isolate default AND lifts the edit
/// storey to the painted upper storey so the three categorical classes (authored-here /
/// exists-below / empty) all read in one shot; anything else keeps the editor's defaults.
/// Honored only when the capture affordance is enabled AND the mode is (forced to)
/// PREFAB. Either value also drives a 2-storey grid with a distinct block painted on the
/// UPPER storey, so the staged capture visibly differs.
const VIEW_ENV_VAR: &str = "GDTF_EDITOR_VIEW";

/// QA / debug-only screenshot-then-exit plugin for the editor.
///
/// Construct it via [`from_env`](EditorCapturePlugin::from_env): it reads `GDTF_EDITOR_SHOT` ONCE
/// and, when unset, [`build`](Plugin::build) registers NOTHING — the affordance is
/// indistinguishable from absent (inert by default). When set, it wires the model-drive systems
/// (its editor-specific DRIVE) then DELEGATES the settle → capture → poll-then-exit steps to the
/// reusable `gdtf_screenshot` primitives (GTW-510):
/// [`settle_then_capture`](gdtf_screenshot::settle_then_capture) /
/// [`poll_then_exit`](gdtf_screenshot::poll_then_exit), keyed off the crate's
/// [`CapturePath`] / [`CaptureProgress`] / [`SettleFrames`] / [`PollCap`] resources.
pub struct EditorCapturePlugin {
    /// The resolved capture path, or `None` when the env var was unset (plugin inert). Typed as the
    /// shared [`CapturePath`] (GTW-510) so the crate's `settle_then_capture` reads it directly.
    path:              Option<CapturePath>,
    /// The forced capture mode (C2.4 — from `GDTF_EDITOR_MODE`), or `None` to keep the editor's
    /// default mode. Read once at construction.
    forced_mode:       Option<ForcedMode>,
    /// The forced TERRAIN kind segment (GTW-574 — from `GDTF_EDITOR_TERRAIN_KIND`), or `None` to
    /// keep the draft's default kind. Read once at construction.
    forced_kind:       Option<ForcedTerrainKind>,
    /// The forced ATTACHMENT-mode loaded item (GTW-669 — from `GDTF_EDITOR_ATTACHMENT`), or
    /// `None` to keep the sorted-first autoload. Read once at construction.
    forced_attachment: Option<ForcedAttachment>,
    /// The forced WEAPON-mode loaded weapon (GTW-670 — from `GDTF_EDITOR_WEAPON`), or
    /// `None` to keep the sorted-first autoload. Read once at construction.
    forced_weapon:     Option<ForcedWeapon>,
    /// The forced preview zoom (C4.11 — from `GDTF_EDITOR_ZOOM`), or `None` to keep the identity
    /// `1.0` scale. Read once at construction.
    forced_zoom:       Option<ForcedZoom>,
    /// The forced prefab storey view (GTW-532 `full` / GTW-594 `isolate` — from
    /// `GDTF_EDITOR_VIEW`), or `None` to keep the editor's defaults. Read once at
    /// construction.
    forced_view:       Option<ForcedView>,
}

impl EditorCapturePlugin {
    /// Reads `GDTF_EDITOR_SHOT` (+ the optional `GDTF_EDITOR_MODE` / `GDTF_EDITOR_TERRAIN_KIND` /
    /// `GDTF_EDITOR_ATTACHMENT` / `GDTF_EDITOR_WEAPON` / `GDTF_EDITOR_ZOOM` / `GDTF_EDITOR_VIEW`)
    /// once and builds the plugin. When
    /// `GDTF_EDITOR_SHOT` is unset the plugin is inert (registers nothing); when set the value is
    /// the PNG output path and the optional vars force the captured mode / TERRAIN kind / loaded
    /// attachment / loaded weapon / zoom / view.
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
            // Inert by default: no env var -> no systems, no resources, normal launch.
            return;
        };
        // GTW-510: seed the shared capture resources the delegated `gdtf_screenshot` primitives
        // read — the resolved path, the egui-safe settle window (30 frames, calibrated for the
        // render-to-texture viewport's first offscreen composite), and the poll cap.
        app.insert_resource(path)
            .insert_resource(SettleFrames::DEFAULT_EGUI)
            .insert_resource(PollCap::DEFAULT)
            .init_resource::<CaptureProgress>()
            .add_systems(OnEnter(EditorState::Editing), reset_progress);
        // C2.4: insert the forced-mode resource (so a QA run can capture a SPECIFIC mode) only when
        // GDTF_EDITOR_MODE named a recognized mode; unset keeps the editor's default mode.
        if let Some(forced) = self.forced_mode {
            app.insert_resource(forced);
        }
        // GTW-574: insert the forced-TERRAIN-kind resource (the Emplacement QA drive) only when
        // GDTF_EDITOR_TERRAIN_KIND named a recognized kind; unset keeps the draft's default kind.
        if let Some(forced) = self.forced_kind {
            app.insert_resource(forced);
        }
        // GTW-669: insert the forced-attachment resource (the named-item QA drive) only when
        // GDTF_EDITOR_ATTACHMENT carried a key; unset keeps the sorted-first autoload.
        if let Some(forced) = self.forced_attachment.clone() {
            app.insert_resource(forced);
        }
        // GTW-670: insert the forced-weapon resource (the named-weapon QA drive) only when
        // GDTF_EDITOR_WEAPON carried a key; unset keeps the sorted-first autoload.
        if let Some(forced) = self.forced_weapon.clone() {
            app.insert_resource(forced);
        }
        // C4.11: insert the forced-zoom resource (the zoom-applied capture variant) only when
        // GDTF_EDITOR_ZOOM parsed a finite float; unset keeps the identity 1.0 scale.
        if let Some(forced) = self.forced_zoom {
            app.insert_resource(forced);
        }
        // GTW-532/GTW-594: insert the forced-view resource (the full-view / isolate capture
        // variants) only when GDTF_EDITOR_VIEW named one; unset keeps the editor's defaults.
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
