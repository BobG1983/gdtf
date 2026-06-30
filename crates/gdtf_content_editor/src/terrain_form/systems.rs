//! The TERRAIN-mode form's **drive systems** (GTW-474): capture each control's commit into the
//! [`TerrainDraft`], reflow the form on a kind change (the C2 footfall gate + the band field),
//! refresh the live RON preview, and write the def on the save press.
//!
//! Every system is param-only (no `&mut World` — bevy-traps #7) and guards the state-scoped
//! [`TerrainDraft`] with `Option<Res<…>>` (bevy-traps #1).

use bevy::prelude::*;
use gdtf_battle_sim::{
    armor::{ArmorHardness, ArmorProtection},
    cover::{CoverHp, HeightBand},
    slab::SlabHp,
};
use gdtf_ui::{
    ActiveButton, DisabledButton, DropdownSelectionChanged, NumericFieldCommitted, SegmentSelected,
    TextFieldCommitted,
};

use super::{
    save::{draft_to_terrain_def, serialize_terrain_def},
    types::{
        ArmorInput, FootfallChoice, HpInput, TerrainArmorHardField, TerrainArmorProtField,
        TerrainBandTabs, TerrainDraft, TerrainFootfallPicker, TerrainGraphicChoice,
        TerrainGraphicPicker, TerrainHpField, TerrainKindChoice, TerrainKindTabs, TerrainNameField,
        TerrainRonPreview, TerrainTagToggle,
    },
};

/// `Update` (in `Editing`): commit the display-name text field into the draft (C2).
pub(crate) fn commit_terrain_name(
    mut commits: MessageReader<TextFieldCommitted>,
    fields: Query<(), With<TerrainNameField>>,
    draft: Option<ResMut<TerrainDraft>>,
) {
    let Some(mut draft) = draft else {
        return;
    };
    for commit in commits.read() {
        if fields.get(commit.field()).is_err() {
            continue;
        }
        draft.set_display_name(commit.value().value().to_owned());
    }
}

/// `Update` (in `Editing`): a kind-tab selection sets the draft kind (C2). The draft setter forces
/// footfall to `None` for a non-slab kind (fail-closed C2); [`gate_footfall_field`] +
/// [`reflow_band_field`] react to the change.
pub(crate) fn apply_terrain_kind(
    mut selections: MessageReader<SegmentSelected>,
    tabs: Query<(), With<TerrainKindTabs>>,
    draft: Option<ResMut<TerrainDraft>>,
) {
    let Some(mut draft) = draft else {
        return;
    };
    for selection in selections.read() {
        if tabs.get(selection.control).is_err() {
            continue;
        }
        if let Some(kind) = TerrainKindChoice::from_segment(*selection.index) {
            draft.set_kind(kind);
        }
    }
}

/// `Update` (in `Editing`): a height-band-tab selection sets the draft band (C2).
pub(crate) fn apply_terrain_band(
    mut selections: MessageReader<SegmentSelected>,
    tabs: Query<(), With<TerrainBandTabs>>,
    draft: Option<ResMut<TerrainDraft>>,
) {
    let Some(mut draft) = draft else {
        return;
    };
    for selection in selections.read() {
        if tabs.get(selection.control).is_err() {
            continue;
        }
        let band = match *selection.index {
            0 => HeightBand::Low,
            1 => HeightBand::Mid,
            _ => HeightBand::High,
        };
        draft.set_height_band(band);
    }
}

/// `Update` (in `Editing`): commit the HP numeric field into BOTH the cover/wall HP pool and the
/// slab HP pool (the form shows ONE "Max HP" field; the projection reads the right pool per kind).
pub(crate) fn commit_terrain_hp(
    mut commits: MessageReader<NumericFieldCommitted<HpInput>>,
    fields: Query<(), With<TerrainHpField>>,
    draft: Option<ResMut<TerrainDraft>>,
) {
    let Some(mut draft) = draft else {
        return;
    };
    for commit in commits.read() {
        if fields.get(commit.field()).is_err() {
            continue;
        }
        let hp = *commit.value().value();
        draft.set_cover_hp(CoverHp::new(hp));
        draft.set_slab_hp(SlabHp::new(hp));
    }
}

/// `Update` (in `Editing`): commit the armor-protection / armor-hardness numeric fields into the
/// draft (C2). One system reads both fields' commits and routes by marker.
pub(crate) fn commit_terrain_armor(
    mut commits: MessageReader<NumericFieldCommitted<ArmorInput>>,
    prot_fields: Query<(), With<TerrainArmorProtField>>,
    hard_fields: Query<(), With<TerrainArmorHardField>>,
    draft: Option<ResMut<TerrainDraft>>,
) {
    let Some(mut draft) = draft else {
        return;
    };
    for commit in commits.read() {
        let value = *commit.value().value();
        if prot_fields.get(commit.field()).is_ok() {
            draft.set_armor_protection(ArmorProtection::new(value));
        } else if hard_fields.get(commit.field()).is_ok() {
            draft.set_armor_hardness(ArmorHardness::new(value));
        }
    }
}

/// The press-edge query filter for a graphic-picker row — a row whose [`Interaction`] changed
/// this frame. A named alias to keep the system signature under clippy's `type_complexity` gate.
type PressedGraphicRow = (Changed<Interaction>, With<TerrainGraphicPicker>);

/// `Update` (in `Editing`): a clicked graphic-role row sets the draft graphic + highlights it
/// (C2 — the per-def graphic picker, never a raw atlas index).
pub(crate) fn select_terrain_graphic(
    mut commands: Commands,
    pressed: Query<(Entity, &Interaction, &TerrainGraphicChoice), PressedGraphicRow>,
    rows: Query<Entity, With<TerrainGraphicPicker>>,
    draft: Option<ResMut<TerrainDraft>>,
) {
    let Some(mut draft) = draft else {
        return;
    };
    let Some((clicked, choice)) = pressed.iter().find_map(|(entity, interaction, choice)| {
        matches!(interaction, Interaction::Pressed).then_some((entity, *choice))
    }) else {
        return;
    };
    draft.set_graphic(choice);
    for entity in &rows {
        if entity == clicked {
            commands.entity(entity).insert(ActiveButton);
        } else {
            commands.entity(entity).remove::<ActiveButton>();
        }
    }
}

/// `Update` (in `Editing`): a footfall-dropdown selection sets the draft footfall (C2 Slab-only —
/// the draft setter ignores it for a non-slab kind, fail-closed).
pub(crate) fn apply_terrain_footfall(
    mut changes: MessageReader<DropdownSelectionChanged<FootfallChoice>>,
    pickers: Query<(), With<TerrainFootfallPicker>>,
    draft: Option<ResMut<TerrainDraft>>,
) {
    let Some(mut draft) = draft else {
        return;
    };
    for change in changes.read() {
        if pickers.get(change.control()).is_err() {
            continue;
        }
        draft.set_footfall(*change.id());
    }
}

/// The press-edge query filter for a tag toggle — a toggle whose [`Interaction`] changed this
/// frame. A named alias to keep the system signature under clippy's `type_complexity` gate.
type PressedTagToggle = (Changed<Interaction>, With<TerrainTagToggle>);

/// `Update` (in `Editing`): a clicked tag toggle flips that tag in the draft's multi-select set +
/// repaints its highlight (C2).
pub(crate) fn toggle_terrain_tag(
    mut commands: Commands,
    pressed: Query<(Entity, &Interaction, &TerrainTagToggle), PressedTagToggle>,
    draft: Option<ResMut<TerrainDraft>>,
) {
    let Some(mut draft) = draft else {
        return;
    };
    for (entity, interaction, toggle) in &pressed {
        if !matches!(interaction, Interaction::Pressed) {
            continue;
        }
        draft.toggle_tag(toggle.tag());
        if draft.has_tag(toggle.tag()) {
            commands.entity(entity).insert(ActiveButton);
        } else {
            commands.entity(entity).remove::<ActiveButton>();
        }
    }
}

/// `Update` (in `Editing`): GATE the footfall dropdown by kind — enable it for a Slab kind,
/// disable it (grey + non-pressable, value forced `None`) otherwise (C2 — footfall offered ONLY
/// for Slab).
///
/// Runs only when the draft CHANGED (covers the first frame + every kind switch). Adds the
/// `gdtf_ui` [`DisabledButton`] marker to the footfall dropdown for a non-slab kind (the
/// interaction layer skips a `DisabledButton`, so it cannot be opened/changed; the paint layer
/// greys it), and removes it for Slab. This makes the Slab-only rule a VISIBLE UI constraint (the
/// Canvas-Centric `DisabledButton` approach from the design).
pub(crate) fn gate_footfall_field(
    mut commands: Commands,
    draft: Option<Res<TerrainDraft>>,
    pickers: Query<(Entity, Has<DisabledButton>), With<TerrainFootfallPicker>>,
) {
    let Some(draft) = draft else {
        return;
    };
    if !draft.is_changed() {
        return;
    }
    let offers = draft.kind().offers_footfall();
    for (entity, disabled) in &pickers {
        if offers && disabled {
            commands.entity(entity).remove::<DisabledButton>();
        } else if !offers && !disabled {
            commands.entity(entity).insert(DisabledButton);
        }
    }
}

/// `Update` (in `Editing`): REFLOW the band segmented control by kind — show it for Wall / Cover
/// (which carry a height band), hide it for Slab (which spans the whole z-boundary), MUTATING the
/// control's [`Node::display`] in place (never despawn — the ui-mutate rule).
pub(crate) fn reflow_band_field(
    draft: Option<Res<TerrainDraft>>,
    mut bands: Query<&mut Node, With<TerrainBandTabs>>,
) {
    let Some(draft) = draft else {
        return;
    };
    if !draft.is_changed() {
        return;
    }
    let want = if draft.kind().has_height_band() {
        Display::Flex
    } else {
        Display::None
    };
    for mut node in &mut bands {
        if node.display != want {
            node.display = want;
        }
    }
}

/// `Update` (in `Editing`): refresh the read-only `.terrain_def.ron` PREVIEW with the current
/// draft (the live preview) — rewritten in place each draft change.
pub(crate) fn refresh_ron_preview(
    draft: Option<Res<TerrainDraft>>,
    mut preview: Query<&mut Text, With<TerrainRonPreview>>,
) {
    let Some(draft) = draft else {
        return;
    };
    if !draft.is_changed() {
        return;
    }
    let Ok(mut text) = preview.single_mut() else {
        return;
    };
    // Project with the (possibly-not-yet-minted) key for the preview only — use the existing key
    // if minted, else the nil sentinel as a placeholder (the real key is minted on save).
    let key = draft
        .uuid()
        .unwrap_or_else(gdtf_battle_sim::terrain::def::TerrainUuid::nil);
    let def = draft_to_terrain_def(&draft, key);
    let body =
        serialize_terrain_def(&def).unwrap_or_else(|err| format!("<serialize error: {err}>"));
    *text = Text::new(body);
}

/// The "Save terrain" press-edge query filter — a button whose [`Interaction`] changed this
/// frame. A named alias to keep the system signature under clippy's `type_complexity` gate.
/// Debug-only (its sole user, [`save_terrain_on_press`], is the gated fs-write trigger).
#[cfg(debug_assertions)]
type PressedSaveButton = (Changed<Interaction>, With<super::types::SaveTerrainButton>);

/// `Update` (in `Editing`): WRITE the terrain def on a "Save terrain" press (C2/C3 — the live
/// trigger).
///
/// Mints the draft's UUID on the first save (C2 — editor-generated, idempotent), resolves the
/// active theme's display name from the [`UuidThemeRegistry`] (the per-theme dir), and calls
/// [`write_terrain`](super::save::write_terrain). On success it updates the read-only UUID text +
/// highlights the button + logs the path; on a typed error it logs + writes nothing. Debug-only
/// (the whole `terrain_form` module is gated). All borrows are `Option` (state-scoped — bevy-traps
/// #1).
#[cfg(debug_assertions)]
pub(crate) fn save_terrain_on_press(
    mut commands: Commands,
    buttons: Query<(Entity, &Interaction), PressedSaveButton>,
    draft: Option<ResMut<TerrainDraft>>,
    session: Option<Res<crate::session::MapEditorSession>>,
    themes: Option<Res<gdtf_battle_sim::level::UuidThemeRegistry>>,
    mut uuid_text: Query<&mut Text, With<super::types::TerrainUuidText>>,
) {
    let Some((button, _)) = buttons
        .iter()
        .find(|(_, interaction)| **interaction == Interaction::Pressed)
    else {
        return;
    };
    let (Some(mut draft), Some(session)) = (draft, session) else {
        return;
    };
    let uuid = draft.ensure_uuid();
    let theme_display = themes
        .as_deref()
        .and_then(|themes| themes.def(&session.theme()))
        .map_or_else(String::new, |def| (*def.display_name).clone());

    match super::save::write_terrain(&draft, uuid, &theme_display) {
        Ok(path) => {
            info!("terrain save: wrote terrain def to `{}`", path.display());
            commands.entity(button).insert(ActiveButton);
            if let Ok(mut text) = uuid_text.single_mut() {
                *text = Text::new(format!("UUID: {}", *uuid));
            }
        }
        Err(err) => error!("terrain save: {err}"),
    }
}
