//! The shell system's per-mode **model-borrow bundles** — one `#[derive(SystemParam)]`
//! struct per Workbench mode that threads more than a couple of borrows, so
//! [`editor_egui_ui`](super::shell::editor_egui_ui)'s argument list stays legible
//! (bundling distinct `SystemParam`s is the standard Bevy pattern for a system that
//! would otherwise take too many). Split out of `shell.rs` at the GTW-636 seam: the
//! bundles change when a MODE's model surface changes, the shell when the PANEL layout
//! does.

use bevy::prelude::*;
use gdtf_battle_presenter::{IsolateView, ViewMode};
use gdtf_battle_sim::{
    armor::ArmorRegistry,
    equipment::attachments::AttachmentRegistry,
    ganger::GangRegistry,
    injuries::{InjuryRegistry, InjuryTables},
    tuning::GangerStatTuning,
    weapon::{MeleeWeaponRegistry, WeaponRegistry},
};
use gdtf_content_families::sprites::SpriteDefRegistry;

use crate::{
    armor_form::ArmorDraft,
    attachment_form::AttachmentDraft,
    canvas::{CanvasZoom, CurrentEditLevel},
    editor_map::EditorMap,
    egui_shell::{prefab::level_rail::RailUiState, sprite_form_ui::SpritePreviewCache},
    gang_form::GangDraft,
    hovered_cell::HoveredCell,
    injury_form::{InjuryDraft, WeightingDraft},
    preview::{target::PreviewTarget, view::PreviewPan},
    sprite_form::SpriteDraft,
    weapon_form::WeaponDraft,
};

/// The state-scoped PREFAB-mode model borrows the shell threads into the PREFAB panels (GTW-515).
/// Every field is `Option` because each resource is state-scoped (inserted `OnEnter(Editing)`,
/// removed `OnExit(Editing)` — bevy-traps #1), so the other modes (which never touch them)
/// tolerate their absence and PREFAB mode no-ops until they exist.
#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct PrefabParams<'w, 's> {
    /// The paintable map model (mutated by click-to-paint).
    pub(super) map:            Option<ResMut<'w, EditorMap>>,
    /// The current edit storey (jumped/scrubbed by the GTW-595 level rail; read by the
    /// viewport paint).
    pub(super) edit_level:     Option<ResMut<'w, CurrentEditLevel>>,
    /// The hovered-cell model (written each frame from the viewport pointer).
    pub(super) hovered:        Option<ResMut<'w, HoveredCell>>,
    /// The owned zoom target (folded from the viewport wheel — set-to-target).
    pub(super) zoom:           Option<ResMut<'w, CanvasZoom>>,
    /// The owned pan target (folded from the viewport right-drag — set-to-target).
    pub(super) pan:            Option<ResMut<'w, PreviewPan>>,
    /// The prefab-viewport view mode (GTW-532) — REUSED from the presenter (the SAME type the
    /// GTW-521 battlescape full-view toggle drives); flipped by the RIGHT-panel view toggle.
    pub(super) view:           Option<ResMut<'w, ViewMode>>,
    /// The orthogonal Isolate toggle (GTW-594) — presenter-owned, editor-defaulted ON with
    /// one onion storey below; flipped by the RIGHT-panel Isolate checkbox. Wins over the
    /// two-state view mode while on (the C3 precedence, decided in the classifier).
    pub(super) isolate:        Option<ResMut<'w, IsolateView>>,
    /// The offscreen preview render target (the viewport draws its egui-registered image).
    pub(super) preview_target: Option<Res<'w, PreviewTarget>>,
    /// The GTW-595 level rail's view-local state (change-keyed thumbnail cache + wheel-scrub
    /// remainder) — a `Local` (the `prefab_save_name` precedent): content-keyed, so it needs no
    /// state-scoped lifecycle.
    pub(super) rail_state:     Local<'s, RailUiState>,
}

/// The GANG-mode model borrows the shell threads into the GANG panels (GTW-636) — the
/// [`PrefabParams`] pattern. Every field is `Option` — the draft is state-scoped
/// (bevy-traps #1) and the registries arrive with the `Load` pass — so the other modes
/// tolerate their absence and GANG mode no-ops until they exist.
#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct GangParams<'w> {
    /// The editable gang working model (the form writes it; the save projects it).
    pub(super) draft:  Option<ResMut<'w, GangDraft>>,
    /// The loaded gang registry (the load `ComboBox` options + the one-shot autoload).
    pub(super) gangs:  Option<Res<'w, GangRegistry>>,
    /// The melee-weapon registry (the melee loadout dropdown's options — GTW-505: the
    /// member model carries a `melee_weapon` key).
    pub(super) melee:  Option<Res<'w, MeleeWeaponRegistry>>,
    /// The armor registry (the armor loadout dropdown's options).
    pub(super) armor:  Option<Res<'w, ArmorRegistry>>,
    /// The GTW-384 stat-derivation weights for the derived readout; the editor loads no
    /// stat tuning, so the panel falls back to the const default when absent (the
    /// retired in-game editor's exact fallback).
    pub(super) tuning: Option<Res<'w, GangerStatTuning>>,
}

/// The ARMOR-mode model borrows the shell threads into the ARMOR panels (GTW-479) — the
/// [`GangParams`] pattern. Every field is `Option` — the draft is state-scoped
/// (bevy-traps #1) and the registry arrives with the `Load` pass — so the other modes
/// tolerate their absence and ARMOR mode no-ops until they exist.
#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct ArmorParams<'w> {
    /// The editable armor working model (the form writes it; the save projects it).
    pub(super) draft:    Option<ResMut<'w, ArmorDraft>>,
    /// The loaded armor registry (the load `ComboBox` options + the one-shot autoload).
    pub(super) registry: Option<Res<'w, ArmorRegistry>>,
}

/// The INJURY-mode model borrows the shell threads into the INJURY panels (GTW-654) —
/// the [`GangParams`] pattern. Every field is `Option` — the two drafts are
/// state-scoped (bevy-traps #1) and the registry + built tables arrive with the
/// `Load` pass — so the other modes tolerate their absence and INJURY mode no-ops
/// until they exist.
#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct InjuryParams<'w> {
    /// The editable injury-def working model (the def form writes it; the save
    /// projects it).
    pub(super) draft:     Option<ResMut<'w, InjuryDraft>>,
    /// The editable weighting-table working model (the weighting section writes
    /// it; its save projects it — GTW-654 C2).
    pub(super) weighting: Option<ResMut<'w, WeightingDraft>>,
    /// The loaded injury registry (the load `ComboBox` + the weighting rows'
    /// injury-name combos + the one-shot autoload).
    pub(super) registry:  Option<Res<'w, InjuryRegistry>>,
    /// The built weighting tables (the context-table load source).
    pub(super) tables:    Option<Res<'w, InjuryTables>>,
}

/// The SPRITE-mode model borrows the shell threads into the SPRITE panels (GTW-664) —
/// the [`GangParams`] pattern. The draft is state-scoped (bevy-traps #1) and the
/// registry arrives with the `Load` pass, so those are `Option` and the other modes
/// tolerate their absence; the asset surfaces the PREVIEW needs (the image store + the
/// server that loads a source path) are `Option` too, so the no-renderer harness (which
/// never runs the egui system anyway) stays constructible.
#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct SpriteParams<'w, 's> {
    /// The editable sprite working model (the form writes it; the save projects it).
    pub(super) draft:         Option<ResMut<'w, SpriteDraft>>,
    /// The loaded sprite-def registry (the load `ComboBox` options + the one-shot
    /// autoload).
    pub(super) registry:      Option<Res<'w, SpriteDefRegistry>>,
    /// The image store — the loaded source image's pixel dims for the anchor preview.
    pub(super) images:        Option<Res<'w, Assets<Image>>>,
    /// The asset server the preview loads the draft's source image path through.
    pub(super) asset_server:  Option<Res<'w, AssetServer>>,
    /// The path-keyed preview/validity cache — a `Local` (the GTW-595 `RailUiState`
    /// precedent): content-keyed, so it needs no state-scoped lifecycle.
    pub(super) preview_cache: Local<'s, SpritePreviewCache>,
}

/// The ATTACHMENT-mode model borrows the shell threads into the ATTACHMENT panels
/// (GTW-669) — the [`GangParams`] pattern. Every field is `Option` — the draft is
/// state-scoped (bevy-traps #1) and the registry arrives with the `Load` pass — so the
/// other modes tolerate their absence and ATTACHMENT mode no-ops until they exist.
#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct AttachmentParams<'w> {
    /// The editable attachment working model (the form writes it; the save projects it).
    pub(super) draft:    Option<ResMut<'w, AttachmentDraft>>,
    /// The loaded attachment registry (the load `ComboBox` options + the one-shot
    /// autoload).
    pub(super) registry: Option<Res<'w, AttachmentRegistry>>,
}

/// The WEAPON-mode model borrows the shell threads into the WEAPON panels (GTW-670) —
/// the [`GangParams`] pattern. Every field is `Option` — the draft is state-scoped
/// (bevy-traps #1) and the registries arrive with the `Load` pass — so the other modes
/// tolerate their absence and WEAPON mode no-ops until they exist. The attachment
/// registry rides here TOO (alongside [`AttachmentParams::registry`] — the
/// [`GangParams`]-holds-`ArmorRegistry` precedent; both are read-only `Res`): it is the
/// option source of the weapon's `attachments:` key combos, which is what makes a
/// dangling key UNAUTHORABLE via the form (GTW-670 C2).
#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct WeaponParams<'w> {
    /// The editable weapon working model (the form writes it; the save projects it).
    pub(super) draft:       Option<ResMut<'w, WeaponDraft>>,
    /// The loaded ranged-weapons registry (the load `ComboBox` options + the one-shot
    /// autoload).
    pub(super) registry:    Option<Res<'w, WeaponRegistry>>,
    /// The loaded attachment registry — the `attachments:` key combos' option source.
    pub(super) attachments: Option<Res<'w, AttachmentRegistry>>,
}
