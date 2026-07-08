//! The [`MapEditorPlugin`] — the editor's single registration seam.
//!
//! Wires the editor's own [`EditorState`] machine, its slim `Load` asset pass
//! ([`register_load`](crate::load::register_load)), the [`Editing`](EditorState::Editing) scene's
//! state-scoped model/resource lifecycle (one
//! [`init_state_scoped_resource`](gdtf_state_scoped::StateScopedResourceAppExt) call per model
//! resource — GTW-575), the
//! standalone editor camera ([`spawn_editor_camera`](crate::camera::spawn_editor_camera)), and —
//! since the GTW-512 egui swap — the single egui Workbench-shell UI system
//! ([`editor_egui_ui`](crate::egui_shell::editor_egui_ui)) in the
//! [`EguiPrimaryContextPass`](bevy_egui::EguiPrimaryContextPass) schedule.
//!
//! It does NOT register any of the game's scene plugins or the battle sim runtime (the GTW-417
//! housing constraint).
//!
//! ## GTW-512: the clean swap off `bevy_ui`
//!
//! The hand-rolled `bevy_ui` shell — `spawn_editor_shell`, the four-region row, the mode-tab
//! segmented control, the theme dropdown / numeric-field widgets, the palette / canvas / right-panel
//! drive systems, and the per-mode `Visibility`-container toggle — is GONE. egui draws the WHOLE
//! shell in ONE system in `EguiPrimaryContextPass`; the kept model resources (the
//! [`EditorMode`](crate::mode::EditorMode), the [`MapEditorSession`](crate::session::MapEditorSession),
//! the [`EditorMap`](crate::editor_map::EditorMap), the level / zoom selectors, the terrain / theme
//! drafts) are still inserted on enter + removed on exit, plus the new
//! [`HoveredCell`](crate::hovered_cell::HoveredCell) model resource (C1.5) the live egui hover and
//! the QA capture both write. The full per-mode FORMS are stubbed in C1 (the egui shell) — the
//! TERRAIN / THEME / PREFAB forms + the texture viewport are the later children C2 / C3 / C4.

use bevy::prelude::*;
use bevy_egui::EguiPrimaryContextPass;
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
    mode::{EditorMode, mode_hotkeys},
    preview::{register_preview, view::PreviewPan},
    right_panel::seed_default_theme,
    session::MapEditorSession,
    sprite_form::SpriteDraft,
    terrain_form::TerrainDraft,
    theme_form::ThemeDraft,
    validate::register_validation,
};

/// The map editor's single plugin: state machine + `Load` pass + `Editing` scene + the egui shell.
///
/// Added by [`MapEditorApp`](crate::MapEditorApp) (and by the headless test) onto an app that
/// already has `DefaultPlugins` + the [`EguiPlugin`](bevy_egui::EguiPlugin). It owns:
///
/// - `init_state::<EditorState>()` — the editor's own two-state lifecycle.
/// - the `Load` pass (the weapon/armor/gang/melee/injury registries, the UUID-keyed
///   terrain/theme registries, and the GTW-663 sprite defs every terrain graphic resolves
///   through since GTW-665) — registered through the SAME generic content-family
///   seams the game uses (GTW-579), WITHOUT pulling the game scene graph.
/// - `OnEnter(Editing)` → the standalone editor camera, plus the FULL
///   state-scoped model/resource lifecycle — the Workbench mode, the authoring session, the
///   paintable map, the level / zoom selectors, the terrain / theme drafts, the hovered-cell
///   model, the preview pan, and the prefab-viewport view mode each register their
///   `OnEnter(Editing)` insert + `OnExit(Editing)` remove through ONE
///   [`init_state_scoped_resource`](gdtf_state_scoped::StateScopedResourceAppExt) call
///   (GTW-575; the state-scoped-resource pattern — bevy-traps #1).
/// - `EguiPrimaryContextPass` (in `Editing`) → [`editor_egui_ui`] draws the whole shell (mode tabs,
///   global theme `ComboBox`, status line, palette/stats placeholder, the active mode's stubbed form,
///   the viewport placeholder). egui systems live in `EguiPrimaryContextPass`, NOT `Update`
///   (bevy-traps: a `Update` `ctx_mut()` call fights the egui begin/end-pass plumbing).
/// - `Update` (in `Editing`) → the UI-agnostic model drives kept from the old shell:
///   [`seed_default_theme`] (seed the session theme to the registry's first theme once it resolves)
///   and [`mode_hotkeys`] (the `1`/`2`/`3` mode hotkeys). The theme SELECTION is now folded into the
///   session by the egui `ComboBox` directly (resolve the chosen theme's default-floor +
///   [`MapEditorSession::select_theme`](crate::session::MapEditorSession::select_theme)) — the verbatim body the old widget-coupled
///   `apply_theme_selection` ran, which is gone now its dropdown message no longer fires.
pub struct MapEditorPlugin;

impl Plugin for MapEditorPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<EditorState>();
        register_load(app);
        // GTW-630: the authoring-time reference-integrity pass — the SAME
        // gdtf_content_families::validate checks the game registers, over the
        // edges the editor loads (theme→terrain + emplacement→weapon + gang
        // equipment, the last since GTW-651), re-armed live on hot-reload so a
        // dangling key surfaces at the edit.
        register_validation(app);

        // GTW-515: disable bevy_egui's auto-attach of the primary context (the editor has TWO
        // cameras now — the window camera + the offscreen prefab preview camera — so auto-attach to
        // the ambiguously-first camera could land on the preview camera and blank the window). The
        // primary context is attached EXPLICITLY to the window camera in `spawn_editor_camera`. Must
        // run before any camera spawns (Startup, before the OnEnter(Editing) camera spawns).
        app.add_systems(Startup, disable_egui_auto_context);

        app.add_systems(OnEnter(EditorState::Editing), spawn_editor_camera);

        // GTW-575: the seventeen `Editing`-scoped MODEL resources register their whole
        // OnEnter-insert + OnExit-remove lifecycle through ONE
        // `init_state_scoped_resource` call each (bevy-traps #1 via the shared
        // `gdtf_state_scoped` seam) — same `OnEnter(Editing)` / `OnExit(Editing)`
        // placement, same seed values the hand-stamped `editor_resources.rs` pairs had.
        //
        // The state-scoped Workbench mode, seeded to the default `Prefab` mode so the
        // editor opens in the existing painter (GTW-474).
        app.init_state_scoped_resource(EditorState::Editing, EditorMode::default);
        // The shared selection state, seeded to the default theme + the full `60×60×8`
        // grid; `seed_default_theme` re-seeds the theme once the registry resolves.
        app.init_state_scoped_resource(EditorState::Editing, MapEditorSession::default);
        // The empty paintable model (GTW-426) — nothing painted; the click-to-paint
        // flow writes it.
        app.init_state_scoped_resource(EditorState::Editing, EditorMap::new);
        // The storey selector (GTW-500 C1), seeded to the ground storey so the editor
        // opens on the same plane the GTW-423 canvas drew.
        app.init_state_scoped_resource(EditorState::Editing, CurrentEditLevel::ground);
        // The zoom factor (GTW-500 C3), seeded to the unzoomed `1.0` (the GTW-423 base
        // cell scale).
        app.init_state_scoped_resource(EditorState::Editing, CanvasZoom::identity);
        // The TERRAIN-mode authoring draft (GTW-474), a fresh default the form's
        // controls seed their initial values from.
        app.init_state_scoped_resource(EditorState::Editing, TerrainDraft::default);
        // The THEME-mode authoring draft (GTW-475), a fresh NEW-theme draft (a minted
        // key, an empty form).
        app.init_state_scoped_resource(EditorState::Editing, ThemeDraft::default);
        // The GANG-mode authoring draft (GTW-636), a pristine form whose one-shot
        // open-with-a-gang autoload is still pending (the shell seeds it from the
        // resolved GangRegistry on the first Gang-mode frame).
        app.init_state_scoped_resource(EditorState::Editing, GangDraft::default);
        // The ARMOR-mode authoring draft (GTW-479), a pristine form whose one-shot
        // open-with-an-armor autoload is still pending (the shell seeds it from the
        // resolved ArmorRegistry on the first Armor-mode frame — the Gang parity).
        app.init_state_scoped_resource(EditorState::Editing, ArmorDraft::default);
        // The INJURY-mode def authoring draft (GTW-654), a pristine form whose
        // one-shot open-with-an-injury autoload is still pending (the shell seeds it
        // from the resolved InjuryRegistry on the first Injury-mode frame — the
        // Gang/Armor parity).
        app.init_state_scoped_resource(EditorState::Editing, InjuryDraft::default);
        // The INJURY-mode weighting-table draft (GTW-654 C2), a pristine form whose
        // one-shot open-with-a-table autoload is still pending (the shell seeds it
        // from the resolved InjuryTables' first canonical category).
        app.init_state_scoped_resource(EditorState::Editing, WeightingDraft::default);
        // The SPRITE-mode authoring draft (GTW-664), a pristine form whose one-shot
        // open-with-a-sprite autoload is still pending (the shell seeds it from the
        // resolved GTW-663 SpriteDefRegistry on the first Sprite-mode frame — the
        // Gang/Armor parity).
        app.init_state_scoped_resource(EditorState::Editing, SpriteDraft::default);
        // The ATTACHMENT-mode authoring draft (GTW-669), a pristine form whose one-shot
        // open-with-an-item autoload is still pending (the shell seeds it from the
        // resolved GTW-619 AttachmentRegistry on the first Attachment-mode frame — the
        // Gang/Armor/Sprite parity).
        app.init_state_scoped_resource(EditorState::Editing, AttachmentDraft::default);
        // GTW-512 C1.5: the hovered-cell model the live egui hover + the QA capture
        // write, seeded empty (nothing hovered).
        app.init_state_scoped_resource(EditorState::Editing, HoveredCell::new);
        // GTW-515 C4.8: the owned pan-offset target, seeded to the origin (the zoom
        // target is the kept CanvasZoom above).
        app.init_state_scoped_resource(EditorState::Editing, PreviewPan::origin);
        // GTW-532: the prefab-viewport ViewMode (REUSED from the presenter — the SAME
        // type the GTW-521 battlescape full-view toggle drives), default DownToActive.
        app.init_state_scoped_resource(EditorState::Editing, ViewMode::default);
        // GTW-594 C2: the orthogonal Isolate toggle — the EDITOR's default is ON with ONE
        // onion storey below (band floor = active), so a higher edit storey shows only its
        // authored content + the categorical ghost below (the GTW-592 fix). While on it
        // WINS over the two-state ViewMode (the C3 precedence); the battlescape's own
        // default stays Off.
        app.init_state_scoped_resource(EditorState::Editing, || {
            IsolateView::On(ContextDepth::new(1))
        });

        // GTW-515 C4.3: the prefab preview render machinery — the offscreen render-target image +
        // the dedicated isolated-render-layer camera (OnEnter/OnExit), the change-driven tile
        // redraw, and the once-per-frame set-to-target zoom/pan apply (Update, in Editing).
        register_preview(app);

        // GTW-512 C1.3: the egui shell — ONE UI system in `EguiPrimaryContextPass` (NOT `Update`),
        // gated to `Editing`. It declares the panels outermost-first with the central panel LAST
        // (egui's load-bearing panel order).
        app.add_systems(
            EguiPrimaryContextPass,
            editor_egui_ui.run_if(in_state(EditorState::Editing)),
        );

        // The UI-agnostic model drives kept from the old shell (no `bevy_ui` dependency): the theme
        // seed + the mode hotkeys. The theme SELECTION is folded into the session by the egui
        // `ComboBox` directly (the verbatim `apply_theme_selection` body), so that widget-coupled
        // drive is not wired.
        app.add_systems(
            Update,
            (
                seed_default_theme,
                mode_hotkeys,
                level_nav_hotkeys,
                view_mode_hotkey,
            )
                .run_if(in_state(EditorState::Editing)),
        );
    }
}
