//! The shell's PRE-PANEL per-mode model-sync / autoload block — split out of
//! `shell.rs` at the GTW-479-flagged natural seam (GTW-654; module-layout warn
//! band): these runners change when a MODE's open-with-content behavior does, the
//! shell when the PANEL layout does. Each runner self-gates on its mode and on its
//! model borrows being present, and each is idempotent under the egui multipass
//! re-run (bevy-traps #8) — the underlying one-shot seeds end their own pending
//! state on the first pass.

use gdtf_battle_sim::{
    armor::ArmorRegistry,
    ganger::GangRegistry,
    injuries::{InjuryRegistry, InjuryTables},
    level::UuidThemeRegistry,
};

use crate::{
    armor_form::ArmorDraft,
    egui_shell::{armor_form_ui, gang_form_ui, injury_form_ui, theme_form_ui},
    gang_form::GangDraft,
    injury_form::{InjuryDraft, WeightingDraft},
    mode::EditorMode,
    session::MapEditorSession,
    theme_form::ThemeDraft,
};

/// C3.2 (GTW-514): when in THEME mode with a theme already selected in the session,
/// auto-load that theme's def into the form so the author edits the live
/// definition. Checked every frame; `resolve_autoload` returns `None` for a nil
/// theme / absent registry, so it no-ops until a real theme resolves. The key
/// comparison avoids redundant reinitialisation across frames — it only loads when
/// the form's current key differs from the session theme (a new selection or a
/// first-enter with a pre-selected theme).
pub(super) fn theme_form_sync(
    mode: EditorMode,
    session: &MapEditorSession,
    themes: Option<&UuidThemeRegistry>,
    draft: &mut ThemeDraft,
) {
    if mode == EditorMode::Theme
        && let Some(themes) = themes
        && let Some(def) = theme_form_ui::resolve_autoload(session.theme(), themes)
        && draft.key() != def.key
    {
        theme_form_ui::load_theme_into_form(draft, def);
    }
}

/// GTW-636: the GANG mode's one-shot open-with-a-gang seed — a still-pristine
/// draft loads the FIRST gang (sorted) from the resolved registry, the retired
/// in-game editor's exact open behavior.
pub(super) fn gang_form_sync(
    mode: EditorMode,
    draft: Option<&mut GangDraft>,
    gangs: Option<&GangRegistry>,
) {
    if mode == EditorMode::Gang
        && let (Some(draft), Some(registry)) = (draft, gangs)
    {
        gang_form_ui::autoload_first_gang(draft, registry);
    }
}

/// GTW-479: the ARMOR mode's one-shot open-with-an-armor seed — the Gang
/// autoload's exact parity twin.
pub(super) fn armor_form_sync(
    mode: EditorMode,
    draft: Option<&mut ArmorDraft>,
    registry: Option<&ArmorRegistry>,
) {
    if mode == EditorMode::Armor
        && let (Some(draft), Some(registry)) = (draft, registry)
    {
        armor_form_ui::autoload_first_armor(draft, registry);
    }
}

/// GTW-654: the INJURY mode's TWO one-shot seeds — the def form opens on the first
/// sorted injury (the Gang/Armor parity) and the weighting section on the first
/// canonical category's current built table.
pub(super) fn injury_form_sync(
    mode: EditorMode,
    draft: Option<&mut InjuryDraft>,
    registry: Option<&InjuryRegistry>,
    weighting: Option<&mut WeightingDraft>,
    tables: Option<&InjuryTables>,
) {
    if mode != EditorMode::Injury {
        return;
    }
    if let (Some(draft), Some(registry)) = (draft, registry) {
        injury_form_ui::autoload_first_injury(draft, registry);
    }
    if let (Some(weighting), Some(tables)) = (weighting, tables) {
        injury_form_ui::autoload_weighting_table(weighting, tables);
    }
}
