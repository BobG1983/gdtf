//! The env-FORCED overrides (`force_capture_*`) — each applies one `GDTF_EDITOR_*`
//! selection (mode / terrain kind / zoom / storey view) to the live model before the
//! settle + shot.

use bevy::prelude::*;
use gdtf_battle_presenter::{ContextDepth, IsolateView, ViewMode};
use gdtf_battle_sim::weapon::{WeaponName, WeaponRegistry};

use super::super::forced::{ForcedMode, ForcedTerrainKind, ForcedView, ForcedZoom};
use crate::{
    EditorMode,
    canvas::{CanvasZoom, CurrentEditLevel, LevelStep},
    session::MapEditorSession,
    terrain_form::TerrainDraft,
};

/// `Update` (in `Editing`, capture-only): FORCE the [`EditorMode`] to the C2.4 [`ForcedMode`]
/// before the settle / screenshot — so the Screenshot-QA can capture a specific Workbench mode
/// (`GDTF_EDITOR_MODE=terrain|theme|prefab|gang|armor`). No-ops when no mode was forced (the resource is
/// absent) or the editor's mode already matches. Both borrows are `Option` (state-scoped —
/// bevy-traps #1); [`set_if_neq`](DetectChangesMut::set_if_neq) keeps an already-matching mode a
/// no-op (idempotent under the egui multipass re-run).
pub(in crate::capture) fn force_capture_mode(
    forced: Option<Res<ForcedMode>>,
    mode: Option<ResMut<EditorMode>>,
) {
    let (Some(forced), Some(mut mode)) = (forced, mode) else {
        return;
    };
    mode.set_if_neq(**forced);
}

/// `Update` (in `Editing`, capture-only): pre-select the TERRAIN form's kind segment to the
/// GTW-574 [`ForcedTerrainKind`] before the settle / screenshot — driving the live
/// [`TerrainDraft`] through the SAME [`set_kind`](TerrainDraft::set_kind) setter the egui
/// segmented row commits. For the Emplacement kind it ALSO pre-selects the FIRST (sorted)
/// [`WeaponRegistry`] key when no mounted weapon is selected yet, so the registry-backed
/// dropdown captures POPULATED with a real weapon name (the AC-7 positive content — egui
/// closures never run headless, so the screenshot is the in-engine evidence).
///
/// No-ops when no kind was forced / the draft is absent (state-scoped — bevy-traps #1);
/// idempotent — the kind write is skipped once matching, and the weapon pre-select only fills
/// an EMPTY selection (a real selection is never overwritten).
pub(in crate::capture) fn force_capture_terrain_kind(
    forced: Option<Res<ForcedTerrainKind>>,
    draft: Option<ResMut<TerrainDraft>>,
    weapons: Option<Res<WeaponRegistry>>,
) {
    let (Some(forced), Some(mut draft)) = (forced, draft) else {
        return;
    };
    if draft.kind() != **forced {
        draft.set_kind(**forced);
    }
    // Emplacement: fill an empty mounted-weapon selection from the live registry (sorted first
    // key — the same stable order the dropdown offers), so the combo shows a real weapon.
    if draft.kind() == crate::terrain_form::TerrainKindChoice::Emplacement
        && draft.mounted_weapon().is_none()
        && let Some(registry) = weapons
    {
        let mut names: Vec<&WeaponName> = registry.keys().collect();
        names.sort_by(|a, b| a.as_str().cmp(b.as_str()));
        if let Some(first) = names.first() {
            draft.set_mounted_weapon(Some((*first).clone()));
        }
    }
}

/// `Update` (in `Editing`, capture-only): FORCE the preview [`CanvasZoom`] to the C4.11
/// [`ForcedZoom`] before the settle / screenshot — so a SECOND capture proves the projection-scale
/// path renders at a non-identity zoom (catches an empty/black viewport at a scaled projection).
/// No-ops when no zoom was forced (the resource is absent) or the zoom already matches. Both
/// borrows are `Option` (state-scoped — bevy-traps #1); [`set_if_neq`](DetectChangesMut::set_if_neq)
/// keeps an already-matching zoom a no-op (idempotent — set-to-target). The
/// `apply_preview_view` system then drives the camera's `OrthographicProjection::scale` from it.
pub(in crate::capture) fn force_capture_zoom(
    forced: Option<Res<ForcedZoom>>,
    zoom: Option<ResMut<CanvasZoom>>,
) {
    let (Some(forced), Some(mut zoom)) = (forced, zoom) else {
        return;
    };
    zoom.set_if_neq(**forced);
}

/// `Update` (in `Editing`, capture-only): FORCE the prefab storey view to the GTW-532 /
/// GTW-594 [`ForcedView`] variant before the settle / screenshot. No-ops when no view was
/// forced (the resource is absent); every borrow is `Option` (state-scoped — bevy-traps
/// #1) and every write is [`set_if_neq`](DetectChangesMut::set_if_neq) (idempotent under
/// the egui multipass re-run). The `redraw_preview_tiles` system then redraws the storey
/// stack from the changed resources.
///
/// - [`ForcedView::Full`] — the GTW-521 whole-stack capture: [`ViewMode::FullView`] AND
///   [`IsolateView::Off`] (Isolate WINS over the two-state mode — GTW-594 C3 — so the
///   full-view capture must lift the editor's Isolate default to show the stack).
/// - [`ForcedView::Isolate`] — the GTW-594 three-class capture: assert the editor default
///   (Isolate ON, one onion below) and LIFT the edit storey to the painted UPPER storey
///   (clamped through the kept level-nav step), so the shot shows authored-here /
///   exists-below / empty at once.
pub(in crate::capture) fn force_capture_view(
    forced: Option<Res<ForcedView>>,
    view: Option<ResMut<ViewMode>>,
    isolate: Option<ResMut<IsolateView>>,
    edit_level: Option<ResMut<CurrentEditLevel>>,
    session: Option<Res<MapEditorSession>>,
) {
    let Some(forced) = forced else {
        return;
    };
    match *forced {
        ForcedView::Full => {
            if let Some(mut view) = view {
                view.set_if_neq(ViewMode::FullView);
            }
            if let Some(mut isolate) = isolate {
                isolate.set_if_neq(IsolateView::Off);
            }
        }
        ForcedView::Isolate => {
            if let Some(mut isolate) = isolate {
                isolate.set_if_neq(IsolateView::On(ContextDepth::new(1)));
            }
            if let (Some(mut edit_level), Some(session)) = (edit_level, session) {
                // Step (not teleport) to the upper storey through the kept clamp — once the
                // 2-storey capture grid applies (`drive_capture_grid_size`, chained after
                // this) the step lands on storey 1; set_if_neq keeps later frames no-ops.
                let upper =
                    CurrentEditLevel::ground().stepped(LevelStep::up(), session.grid_size());
                edit_level.set_if_neq(upper);
            }
        }
    }
}
