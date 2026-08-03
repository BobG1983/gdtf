use bevy::prelude::*;
use gdtf_battle_presenter::{ContextDepth, IsolateView, ViewMode};
use gdtf_battle_sim::{
    equipment::attachments::AttachmentRegistry,
    weapon::{MeleeWeaponRegistry, WeaponName, WeaponRegistry},
};

use super::super::forced::{
    ForcedAttachment, ForcedMeleeWeapon, ForcedMode, ForcedTerrainKind, ForcedView, ForcedWeapon,
    ForcedZoom,
};
use crate::{
    EditorMode,
    attachment_form::AttachmentDraft,
    canvas::{CanvasZoom, CurrentEditLevel, LevelStep},
    melee_weapon_form::MeleeWeaponDraft,
    session::MapEditorSession,
    terrain_form::TerrainDraft,
    weapon_form::WeaponDraft,
};

pub(in crate::capture) fn force_capture_mode(
    forced: Option<Res<ForcedMode>>,
    mode: Option<ResMut<EditorMode>>,
) {
    let (Some(forced), Some(mut mode)) = (forced, mode) else {
        return;
    };
    mode.set_if_neq(**forced);
}

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

pub(in crate::capture) fn force_capture_attachment(
    forced: Option<Res<ForcedAttachment>>,
    draft: Option<ResMut<AttachmentDraft>>,
    registry: Option<Res<AttachmentRegistry>>,
) {
    let (Some(forced), Some(mut draft), Some(registry)) = (forced, draft, registry) else {
        return;
    };
    if draft.name() == forced.as_str() {
        return;
    }
    if let Some(spec) = registry.spec(&forced) {
        let spec = spec.clone();
        draft.load_attachment(&forced, &spec);
    }
}

pub(in crate::capture) fn force_capture_weapon(
    forced: Option<Res<ForcedWeapon>>,
    draft: Option<ResMut<WeaponDraft>>,
    registry: Option<Res<WeaponRegistry>>,
) {
    let (Some(forced), Some(mut draft), Some(registry)) = (forced, draft, registry) else {
        return;
    };
    if draft.name() == forced.as_str() {
        return;
    }
    if let Some(spec) = registry.spec(&forced) {
        let spec = spec.clone();
        draft.load_weapon(&forced, &spec);
    }
}

pub(in crate::capture) fn force_capture_melee_weapon(
    forced: Option<Res<ForcedMeleeWeapon>>,
    draft: Option<ResMut<MeleeWeaponDraft>>,
    registry: Option<Res<MeleeWeaponRegistry>>,
) {
    let (Some(forced), Some(mut draft), Some(registry)) = (forced, draft, registry) else {
        return;
    };
    if draft.name() == forced.as_str() {
        return;
    }
    if let Some(spec) = registry.spec(&forced) {
        let spec = spec.clone();
        draft.load_melee_weapon(&forced, &spec);
    }
}

pub(in crate::capture) fn force_capture_zoom(
    forced: Option<Res<ForcedZoom>>,
    zoom: Option<ResMut<CanvasZoom>>,
) {
    let (Some(forced), Some(mut zoom)) = (forced, zoom) else {
        return;
    };
    zoom.set_if_neq(**forced);
}

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
                let upper =
                    CurrentEditLevel::ground().stepped(LevelStep::up(), session.grid_size());
                edit_level.set_if_neq(upper);
            }
        }
    }
}
