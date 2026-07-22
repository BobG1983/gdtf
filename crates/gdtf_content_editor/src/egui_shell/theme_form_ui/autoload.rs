//! The C3.2 load-existing path — the theme→draft form load and the session autoload
//! resolution the shell drives on entering THEME mode / on a top-bar theme pick.

use gdtf_battle_sim::level::{ThemeUuid, UuidThemeDef, UuidThemeRegistry};

use crate::theme_form::ThemeDraft;

/// Load an existing [`UuidThemeDef`] into the form — the C3.2 "load-existing" affordance. Called
/// by the shell when the top-bar theme `ComboBox` selects a theme while in THEME mode (and on
/// entering THEME mode with a theme already selected). Replaces the draft with one built from the
/// def's parts via [`ThemeDraft::from_parts`] (reused verbatim — C3.3).
pub(crate) fn load_theme_into_form(draft: &mut ThemeDraft, def: &UuidThemeDef) {
    *draft = ThemeDraft::from_parts(
        def.key,
        (*def.display_name).clone(),
        def.terrain.clone(),
        def.default_floor,
    );
}

// GTW-574 C7: the verbatim `sim_kind_label` copy this file carried is GONE — the library rows
// import the ONE label fn from `crate::theme_form` (re-exported from its `resolve` submodule),
// which since GTW-574 matches exhaustively over the canonical `TerrainPieceKind` projection.

/// Resolve which [`ThemeUuid`] to auto-load into the form when entering THEME mode — the session's
/// selected theme if it is not the nil sentinel and the registry resolves it, else [`None`].
/// Returns the [`UuidThemeDef`] reference so the caller passes it straight to
/// [`load_theme_into_form`] (no double-lookup). Pure, so tests can exercise the resolution path.
pub(crate) fn resolve_autoload(
    session_theme: ThemeUuid,
    themes: &UuidThemeRegistry,
) -> Option<&UuidThemeDef> {
    if *session_theme.is_nil() {
        return None;
    }
    themes.def(&session_theme)
}
