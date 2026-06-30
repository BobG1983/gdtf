//! The THEME-mode form's **read-only render systems + the save trigger** (GTW-475): the C3
//! single-source-of-truth proof (the resolved-default-floor readout + HP bar), the read-only KEY
//! text, the live `.terrain_theme.ron` preview, and the debug-only Save-theme press.
//!
//! Split out of [`systems`](super::systems) (the input/drive systems) to keep each file within the
//! size caps; these systems are all read-only over the [`ThemeDraft`] (no draft mutation) apart
//! from the save trigger, which only reads it. Every system is param-only (no `&mut World` —
//! bevy-traps #7) and guards the state-scoped [`ThemeDraft`] with `Option<Res<…>>` (bevy-traps #1).

use bevy::prelude::*;
use gdtf_battle_sim::terrain::def::{TerrainDef, TerrainDefRegistry, TerrainSimKind};
use gdtf_ui::{ActiveButton, FillFraction, ProgressBarFill, set_progress_bar};

use super::{
    save::{draft_to_theme_def, serialize_theme_def},
    types::{
        ThemeDraft, ThemeKeyText, ThemeResolvedHpBar, ThemeResolvedStatsText, ThemeRonPreview,
    },
};

/// `Update` (in `Editing`): refresh the read-only KEY text with the draft's key — rewritten on a
/// load / New-theme reset (C4), so the shown key always matches the draft.
pub(crate) fn refresh_theme_key_text(
    draft: Option<Res<ThemeDraft>>,
    mut key_text: Query<&mut Text, With<ThemeKeyText>>,
) {
    let Some(draft) = draft else {
        return;
    };
    if !draft.is_changed() {
        return;
    }
    let Ok(mut text) = key_text.single_mut() else {
        return;
    };
    *text = Text::new(format!("Key: {}", *draft.key()));
}

/// `Update` (in `Editing`): refresh the C3 RESOLVED-STATS readout — for the draft's selected
/// default-floor terrain, RESOLVE it against the [`TerrainDefRegistry`] and show its sim kind +
/// HP / armor + drive the HP bar's fill. The single-source-of-truth proof: the theme stored only a
/// [`TerrainUuid`](gdtf_battle_sim::terrain::def::TerrainUuid); the stats are RESOLVED here, never
/// inlined.
///
/// Runs when the draft CHANGED. Rewrites the [`ThemeResolvedStatsText`] node + the
/// [`ThemeResolvedHpBar`] fill in place (mutate, never respawn). With no floor / an unresolvable
/// floor the readout shows a placeholder and the bar empties.
pub(crate) fn refresh_resolved_stats(
    draft: Option<Res<ThemeDraft>>,
    terrain: Option<Res<TerrainDefRegistry>>,
    mut readout: Query<&mut Text, With<ThemeResolvedStatsText>>,
    bars: Query<Entity, With<ThemeResolvedHpBar>>,
    children: Query<&Children>,
    mut fills: Query<&mut Node, With<ProgressBarFill>>,
) {
    let (Some(draft), Some(terrain)) = (draft, terrain) else {
        return;
    };
    if !draft.is_changed() {
        return;
    }
    let resolved = draft
        .default_floor()
        .and_then(|floor| terrain.def(&floor).map(resolved_stats));
    let (summary, fraction) =
        resolved.unwrap_or_else(|| ("Pick a default floor to resolve its stats.".to_owned(), 0.0));
    if let Ok(mut text) = readout.single_mut() {
        *text = Text::new(summary);
    }
    // Drive the HP bar's fill (the resolved HP fraction against a generous structural ceiling)
    // via the public widget mutator — mutate-in-place, never respawn (the ui-mutate rule).
    let fraction = FillFraction::new(fraction);
    for track in &bars {
        set_progress_bar(track, fraction, &children, &mut fills);
    }
}

/// The generous structural-HP ceiling the resolved-stats bar fraction is taken against — matches
/// the TERRAIN form's HP clamp ceiling so a structural pool reads as a sensible bar fill (C3).
const HP_BAR_CEILING: f32 = 1000.0;

/// Resolve a terrain def into a `(human summary, HP fraction)` for the C3 readout — its kind, HP,
/// armor, hardness (+ band for Wall / Cover), and the HP fraction against [`HP_BAR_CEILING`].
/// Pure, so the C7 test pins the resolution against a registry fixture.
#[must_use]
pub(crate) fn resolved_stats(def: &TerrainDef) -> (String, f32) {
    // `**hp` is a `u32` structural pool; the documented small-magnitude cast is clippy-clean
    // under the workspace `cast_precision_loss` allow (the presenter `framing.rs` precedent).
    let (kind, hp, protection, hardness, band) = match &def.sim_kind {
        TerrainSimKind::Wall {
            hp,
            armor_protection,
            armor_hardness,
            height_band,
        } => (
            "Wall",
            **hp as f32,
            **armor_protection,
            **armor_hardness,
            Some(format!("{height_band:?}")),
        ),
        TerrainSimKind::Cover {
            hp,
            armor_protection,
            armor_hardness,
            height_band,
        } => (
            "Cover",
            **hp as f32,
            **armor_protection,
            **armor_hardness,
            Some(format!("{height_band:?}")),
        ),
        TerrainSimKind::Slab {
            hp,
            armor_protection,
            armor_hardness,
        } => (
            "Slab",
            **hp as f32,
            **armor_protection,
            **armor_hardness,
            None,
        ),
    };
    let name = (*def.display_name).clone();
    let band_line = band.map_or_else(String::new, |b| format!("\nBand: {b}"));
    let summary = format!(
        "{name}\n{kind}\nHP: {hp:.0}\nArmor: {protection}\nHardness: {hardness}{band_line}"
    );
    (summary, hp / HP_BAR_CEILING)
}

/// `Update` (in `Editing`): refresh the read-only `.terrain_theme.ron` PREVIEW with the current
/// draft (the live preview — C3), rewritten in place each draft change.
pub(crate) fn refresh_theme_ron_preview(
    draft: Option<Res<ThemeDraft>>,
    mut preview: Query<&mut Text, With<ThemeRonPreview>>,
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
    let def = draft_to_theme_def(&draft, draft.key());
    let body = serialize_theme_def(&def).unwrap_or_else(|err| format!("<serialize error: {err}>"));
    *text = Text::new(body);
}

/// The "Save theme" press-edge query filter — a button whose [`Interaction`] changed this frame.
/// A named alias to keep the system signature under clippy's `type_complexity` gate. Debug-only
/// (its sole user, [`save_theme_on_press`], is the gated fs-write trigger).
#[cfg(debug_assertions)]
type PressedSaveButton = (Changed<Interaction>, With<super::types::SaveThemeButton>);

/// `Update` (in `Editing`): WRITE the theme def on a "Save theme" press (C5 — the live trigger).
///
/// Writes the draft via [`write_theme`](super::save::write_theme) keyed by the draft's key (minted
/// for a new theme / loaded when editing — C4), honoring the C6 default-floor rule. On success it
/// highlights the button + logs the path; on a typed error it logs + writes nothing. Debug-only
/// (the whole save trigger is gated, the GTW-474 terrain-save precedent). All borrows are `Option`
/// (state-scoped — bevy-traps #1).
#[cfg(debug_assertions)]
pub(crate) fn save_theme_on_press(
    mut commands: Commands,
    buttons: Query<(Entity, &Interaction), PressedSaveButton>,
    draft: Option<Res<ThemeDraft>>,
) {
    let Some((button, _)) = buttons
        .iter()
        .find(|(_, interaction)| **interaction == Interaction::Pressed)
    else {
        return;
    };
    let Some(draft) = draft else {
        return;
    };
    match super::save::write_theme(&draft, draft.key()) {
        Ok(path) => {
            info!("theme save: wrote theme def to `{}`", path.display());
            commands.entity(button).insert(ActiveButton);
        }
        Err(err) => error!("theme save: {err}"),
    }
}
