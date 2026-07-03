//! The egui THEME-mode authoring form (GTW-514 C3) — the real theme form that replaces the
//! C1 stub in the right panel + left stats + central (RON-preview) regions of the egui shell.
//!
//! This module is the egui DRAW + the debug-only save press for the THEME mode. It REUSES the
//! [`theme_form`](crate::theme_form) model + save VERBATIM (C3.3): every control reads / writes
//! the state-scoped [`ThemeDraft`] resource through its existing accessors / setters, and the
//! save button calls the existing [`write_theme`](crate::theme_form::write_theme) — round-tripping
//! the GTW-489 theme loader. The egui draw lives in
//! [`EguiPrimaryContextPass`](bevy_egui::EguiPrimaryContextPass) (bevy-traps #8) via the caller
//! [`editor_egui_ui`](super::shell::editor_egui_ui); the controls mutate the draft in place
//! (no message round-trip — egui is immediate-mode), so the mutations are idempotent under the
//! multipass re-run (bevy-traps #8 fact (b)).
//!
//! ## Layout across the shell panels (GTW-530 — reworked emphasis)
//!
//! GTW-530 REWORKS the GTW-514 THEME layout so the terrain LIBRARY is the primary focus and the
//! `.terrain_theme.ron` preview is REMOVED entirely (unlike the TERRAIN tab's GTW-534 DEMOTION —
//! here the author never wants the RON, so the central space it held is reclaimed for the library):
//!
//! - CENTRAL primary panel — the [`terrain_library_panel`]: the terrain multi-select library, now
//!   the largest / most-prominent region. Each row renders `[sprite thumbnail] name [Kind]` — the
//!   sprite resolved via the SHARED [`sprite_thumb`](crate::egui_shell::sprite_thumb) helper
//!   (GTW-516) over the terrain's [`terrain_atlas_index`](crate::terrain_graphics::terrain_atlas_index),
//!   NO hardcoded index. The check/uncheck multi-select (fail-closed default floor) is preserved.
//! - LEFT palette panel — the resolved-stats readout for the current default-floor terrain (or a
//!   placeholder when none is selected), reusing [`resolved_stats`](crate::theme_form::resolved_stats)
//!   (the C3 stat proof) — a text summary + an [`egui::ProgressBar`] HP bar.
//! - RIGHT mode-form panel — the field stack: display name, the default-floor [`ComboBox`]
//!   (SLAB-ONLY via [`slab_floor_candidates`](crate::theme_form::slab_floor_candidates) — GTW-530
//!   C3), the read-only UUID, the New-theme button, and the debug-only Save button. The terrain
//!   library is NO LONGER here (it is the central primary region — GTW-530 C2).
//! - The `.terrain_theme.ron` preview is GONE (GTW-530 C1).
//!
//! ## Load-existing (C3.2)
//!
//! When the global theme `ComboBox` in the top bar selects a theme while in THEME mode, the shell
//! calls [`load_theme_into_form`] so the draft shows the selected theme's current definition. On
//! entering THEME mode with a theme already selected, the shell does the same check.

use bevy_egui::egui;
use gdtf_battle_presenter::TileRoles;
use gdtf_battle_sim::{
    level::{ThemeUuid, UuidThemeDef, UuidThemeRegistry},
    terrain::def::{TerrainDefRegistry, TerrainUuid},
};

use crate::{
    egui_shell::sprite_thumb,
    terrain_graphics::terrain_atlas_index,
    theme_form::{ThemeDraft, resolved_stats, sim_kind_label, slab_floor_candidates},
};

/// The placeholder text shown in the left stats panel when no default-floor terrain is selected
/// or the registry is absent.
const NO_FLOOR_STATS: &str = "Select a terrain\nas the default floor\nto see resolved stats.";

/// The placeholder UUID line shown for a not-yet-meaningful theme key (e.g. before any load).
const KEY_PREFIX: &str = "UUID: ";

/// Draw the THEME-mode LEFT stats panel (C3.1) — the resolved-stats readout for the current
/// default-floor terrain. Reuses [`resolved_stats`] verbatim (C3.3): when the draft has a
/// default-floor key and the registry resolves it, shows the terrain's kind + HP + armor + band
/// summary and an [`egui::ProgressBar`] HP bar. When no floor is selected or the registry is
/// absent, shows a placeholder (never a panic).
pub(crate) fn stats_panel(
    ui: &mut egui::Ui,
    draft: &ThemeDraft,
    terrain: Option<&TerrainDefRegistry>,
) {
    ui.heading("Floor stats");
    ui.separator();
    let resolved = draft
        .default_floor()
        .zip(terrain)
        .and_then(|(key, reg)| reg.def(&key))
        .map(resolved_stats);
    if let Some((summary, fraction)) = resolved {
        ui.label(&summary);
        ui.separator();
        ui.add(egui::ProgressBar::new(fraction.clamp(0.0, 1.0)).text("HP"));
    } else {
        ui.label(NO_FLOOR_STATS);
    }
}

/// Draw the THEME-mode FIELD STACK into the RIGHT mode-form panel (GTW-530 C2/C3) — display name,
/// the default-floor [`ComboBox`] (SLAB-ONLY via [`slab_floor_candidates`] — GTW-530 C3), the
/// read-only UUID key, the New-theme button (mints a fresh draft), and the debug-only Save button.
///
/// The terrain library multi-select is NO LONGER drawn here (GTW-530 C2): it is the CENTRAL primary
/// region ([`terrain_library_panel`]) so the largest space showcases the terrain + its sprites.
///
/// Every control reads / writes the [`ThemeDraft`] through its existing accessors / setters
/// (C3.3): the floor combo routes through [`ThemeDraft::set_default_floor`] (which ignores a key
/// not in the palette — fail-closed C6).
pub(crate) fn field_stack(
    ui: &mut egui::Ui,
    draft: &mut ThemeDraft,
    terrain: Option<&TerrainDefRegistry>,
) {
    ui.heading("Theme");
    ui.separator();

    name_field(ui, draft);
    floor_combo(ui, draft, terrain);
    uuid_text(ui, draft);

    ui.separator();
    new_theme_button(ui, draft);

    #[cfg(debug_assertions)]
    {
        ui.separator();
        save_button(ui, draft);
    }
}

/// The display-name text field — a single-line edit committed straight into the draft (C3.1).
fn name_field(ui: &mut egui::Ui, draft: &mut ThemeDraft) {
    ui.label("Display name");
    let mut name = draft.display_name().to_owned();
    if ui.text_edit_singleline(&mut name).changed() {
        draft.set_display_name(name);
    }
}

/// Draw the THEME-mode CENTRAL PRIMARY region (GTW-530 C2) — the terrain multi-select LIBRARY, now
/// the largest / most-prominent region of the theme tab (formerly a strip inside the right-panel
/// field stack; GTW-530 promotes it to the central primary space the removed RON preview vacated —
/// C1).
///
/// A [`ScrollArea`] of rows, one per registered terrain, sorted by display name so the order is
/// deterministic (the registry is a `HashMap`). Each row renders `[sprite thumbnail] name [Kind]`:
/// the SPRITE via the SHARED [`sprite_thumb`] helper (GTW-516) over the terrain's
/// [`terrain_atlas_index`](crate::terrain_graphics::terrain_atlas_index) — the SAME resolution the
/// battlescape + the TERRAIN picker use, NO hardcoded index — then a selectable multi-select
/// checkbox carrying `"name [Kind]"` (Kind = Wall / Cover / Slab / Emplacement). A check / uncheck
/// routes through
/// [`ThemeDraft::toggle_terrain`], which fail-closes the default floor if the toggled terrain was
/// the chosen floor (the C6 rule) — the multi-select behavior is PRESERVED across the relocation.
///
/// When no registry is present (registries not yet resolved), the area shows a loading marker
/// rather than panicking. An absent [`TileRoles`] / unregistered sheet leaves the sprite unresolved
/// and the shared helper draws a fixed-size blank spacer so the row layout stays stable. The sort
/// is rebuilt from the raw `defs()` iterator every draw (deterministic — the registry count / key
/// change drives a natural re-sort). `sheet_id` is the terrain sheet's egui texture id (resolved by
/// the shell before the draw); `roles` is the presenter's role table.
pub(crate) fn terrain_library_panel(
    ui: &mut egui::Ui,
    draft: &mut ThemeDraft,
    terrain: Option<&TerrainDefRegistry>,
    roles: Option<&TileRoles>,
    sheet_id: Option<egui::TextureId>,
) {
    ui.heading("Terrain library");
    ui.separator();
    let Some(reg) = terrain else {
        ui.label("(loading…)");
        return;
    };
    // Sort by display name for a deterministic, reader-friendly list.
    let mut entries: Vec<(TerrainUuid, String)> = reg
        .defs()
        .map(|(key, def)| {
            let label = format!("{}  [{}]", *def.display_name, sim_kind_label(&def.sim_kind));
            (*key, label)
        })
        .collect();
    entries.sort_by(|(_, a), (_, b)| a.cmp(b));

    egui::ScrollArea::vertical()
        .id_salt("theme_terrain_library")
        .show(ui, |ui| {
            for (key, label) in entries {
                terrain_library_row(ui, draft, reg, roles, sheet_id, key, &label);
            }
        });
}

/// Draw ONE terrain-library row — `[sprite thumbnail] name [Kind]` on a single horizontal line
/// (GTW-530 C2).
///
/// The sprite is the terrain's atlas index resolved via
/// [`terrain_atlas_index`](crate::terrain_graphics::terrain_atlas_index) (the presenter's
/// resolution — no hardcoded index) and drawn by the shared [`sprite_thumb::draw_thumb`] (which
/// allocates a fixed-size blank spacer when the index / sheet is unavailable, keeping rows aligned
/// — never a panic). The multi-select is a checkbox carrying the `"name [Kind]"` label; a toggle
/// routes through [`ThemeDraft::toggle_terrain`] (fail-closed default floor — C6). Resolving the
/// index needs both the registry + the role table; when [`TileRoles`] is absent the sprite is left
/// unresolved (blank spacer) but the checkbox still works.
fn terrain_library_row(
    ui: &mut egui::Ui,
    draft: &mut ThemeDraft,
    reg: &TerrainDefRegistry,
    roles: Option<&TileRoles>,
    sheet_id: Option<egui::TextureId>,
    key: TerrainUuid,
    label: &str,
) {
    ui.horizontal(|ui| {
        // Resolve the sprite the way the presenter + TERRAIN picker do (no hardcoded index): the
        // terrain's graphic role → atlas index. `None` (absent role table / out-of-vocabulary role)
        // draws a fixed-size blank spacer so the row still lines up.
        let index = roles.and_then(|roles| terrain_atlas_index(reg, roles, &key));
        sprite_thumb::draw_thumb(ui, index, sheet_id);
        let mut checked = draft.has_terrain(key);
        if ui.checkbox(&mut checked, label).changed() {
            draft.toggle_terrain(key);
        }
    });
}

/// The default-floor [`ComboBox`] (GTW-530 C3) — offers ONLY the draft's own SLAB-kind terrain as
/// candidates (via [`slab_floor_candidates`]), with the current default floor pre-selected. The
/// walkable floor is always a slab, so no Wall / Cover terrain ever appears in this picker.
/// Choosing a row routes through [`ThemeDraft::set_default_floor`], which ignores it unless the
/// terrain is in the palette (fail-closed C6). When no slab is selected the combo shows `"(none)"`
/// and is disabled. Requires the terrain registry to resolve display names; falls back to a
/// read-only label when absent.
fn floor_combo(ui: &mut egui::Ui, draft: &mut ThemeDraft, terrain: Option<&TerrainDefRegistry>) {
    ui.label("Default floor");
    let Some(reg) = terrain else {
        ui.label("(loading…)");
        return;
    };
    let candidates = slab_floor_candidates(draft, reg);
    if candidates.is_empty() {
        ui.add_enabled_ui(false, |ui| {
            egui::ComboBox::from_id_salt("theme_floor_combo")
                .selected_text("(none)")
                .show_ui(ui, |_ui| {});
        });
        return;
    }
    let current_floor = draft.default_floor();
    let preview = current_floor
        .and_then(|key| candidates.iter().find(|(k, _)| *k == key))
        .map_or_else(|| "(select…)".to_owned(), |(_, label)| label.clone());
    let mut chosen = current_floor;
    let mut clicked = false;
    egui::ComboBox::from_id_salt("theme_floor_combo")
        .selected_text(preview)
        .show_ui(ui, |ui| {
            for (key, label) in &candidates {
                let is_selected = Some(*key) == current_floor;
                if ui.selectable_label(is_selected, label).clicked() {
                    chosen = Some(*key);
                    clicked = true;
                }
            }
        });
    if clicked && let Some(key) = chosen {
        draft.set_default_floor(key);
    }
}

/// The read-only UUID line (C3.1) — the draft's theme key shown as a formatted UUID.
fn uuid_text(ui: &mut egui::Ui, draft: &ThemeDraft) {
    let key_str = format!("{}{}", KEY_PREFIX, *draft.key());
    ui.label(key_str);
}

/// The New-theme button (C3.1) — on press, replaces the draft with a freshly minted
/// [`ThemeDraft::new_theme`] (a new UUID, empty name, no terrain, no floor). Idempotent under
/// the egui multipass re-run (the second press is a distinct click event, not a duplicate).
fn new_theme_button(ui: &mut egui::Ui, draft: &mut ThemeDraft) {
    if ui.button("New theme").clicked() {
        *draft = ThemeDraft::new_theme();
    }
}

/// The debug-only Save button (C3.1) — on press it validates the draft (
/// [`validate_for_save`](crate::theme_form::validate_for_save)) and calls the existing
/// [`write_theme`](crate::theme_form::write_theme) (C3.3). On a typed error it logs and writes
/// nothing (never a panic). Debug-only — the whole theme save path is gated
/// `#[cfg(debug_assertions)]`.
#[cfg(debug_assertions)]
fn save_button(ui: &mut egui::Ui, draft: &ThemeDraft) {
    if !ui.button("Save theme").clicked() {
        return;
    }
    let key = draft.key();
    match crate::theme_form::write_theme(draft, key) {
        Ok(path) => bevy::log::info!("theme save: wrote theme def to `{}`", path.display()),
        Err(err) => bevy::log::error!("theme save: {err}"),
    }
}

// GTW-530 C1: the `.terrain_theme.ron` preview was REMOVED from the THEME tab entirely. Authors do
// not read the raw RON here, so the central space it held is reclaimed for the terrain library
// ([`terrain_library_panel`]) — the tab's primary focus (C2). (This is the deliberate difference
// from the TERRAIN tab's GTW-534, which only DEMOTED its preview to a side strip.)

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
    if session_theme.is_nil() {
        return None;
    }
    themes.def(&session_theme)
}

/// C3.5 — egui-layer pure-function tests for [`resolve_autoload`] and [`load_theme_into_form`].
///
/// These cover the NEW egui-layer fns that have no equivalent in `theme_form/tests.rs` (which
/// covers the model-layer: projection, round-trip, validation, resolution, floor-candidates).
/// The round-trip identity test is NOT duplicated — it is already covered by
/// `theme_def_round_trips_through_the_loader_parser` in `crate::theme_form::tests`.
#[cfg(test)]
mod tests {
    use gdtf_battle_sim::{
        armor::{ArmorHardness, ArmorProtection},
        cover::{CoverHp, HeightBand},
        level::{ThemeDisplayName, ThemeUuid, UuidThemeDef, UuidThemeRegistry},
        slab::SlabHp,
        terrain::{
            def::{
                TerrainDef, TerrainDefRegistry, TerrainDisplayName, TerrainPresenterKind,
                TerrainSimKind, TerrainUuid,
            },
            piece::TerrainGraphicKey,
        },
    };

    use super::{load_theme_into_form, resolve_autoload};
    use crate::theme_form::ThemeDraft;

    /// A deterministic [`ThemeUuid`] from a small integer — no random UUIDs in tests.
    fn theme_key(n: u128) -> ThemeUuid {
        ThemeUuid::new(bevy::asset::uuid::Uuid::from_u128(0x0001_84c3_0000 + n))
    }

    /// A deterministic [`TerrainUuid`] from a small integer.
    fn terrain_key(n: u128) -> TerrainUuid {
        TerrainUuid::new(bevy::asset::uuid::Uuid::from_u128(0x0001_84c3_1000 + n))
    }

    /// A minimal Slab [`TerrainDef`] with the given key and display name — constructed with
    /// real types, no stubs.
    fn slab_def(key: TerrainUuid, name: &str) -> TerrainDef {
        TerrainDef {
            key,
            display_name: TerrainDisplayName::new(name.to_owned()),
            sim_kind: TerrainSimKind::Slab {
                hp:               SlabHp::new(100),
                armor_protection: ArmorProtection::new(4),
                armor_hardness:   ArmorHardness::new(2),
            },
            presenter_kind: TerrainPresenterKind::Slab {
                graphic_name: TerrainGraphicKey::new("slab".to_owned()),
                footfall:     None,
            },
            tags: Vec::new(),
            on_death: None,
        }
    }

    /// A minimal Wall [`TerrainDef`] with the given key and display name.
    fn wall_def(key: TerrainUuid, name: &str) -> TerrainDef {
        TerrainDef {
            key,
            display_name: TerrainDisplayName::new(name.to_owned()),
            sim_kind: TerrainSimKind::Wall {
                hp:               CoverHp::new(60),
                armor_protection: ArmorProtection::new(8),
                armor_hardness:   ArmorHardness::new(4),
                height_band:      HeightBand::High,
            },
            presenter_kind: TerrainPresenterKind::Wall {
                graphic_name: TerrainGraphicKey::new("wall".to_owned()),
            },
            tags: Vec::new(),
            on_death: None,
        }
    }

    /// A one-theme [`UuidThemeRegistry`] with a fixed display name — gives tests a real registry
    /// without touching the filesystem or any global state.
    fn single_theme_registry(
        t_key: ThemeUuid,
        terrain: Vec<TerrainUuid>,
        floor: TerrainUuid,
    ) -> UuidThemeRegistry {
        let def = UuidThemeDef {
            key: t_key,
            display_name: ThemeDisplayName::new("Hive District".to_owned()),
            default_floor: floor,
            terrain,
        };
        UuidThemeRegistry::new([(t_key, def)])
    }

    // ── C3.5(a) resolve_autoload ──────────────────────────────────────────────────────────────────

    /// C3.5(a) — the NIL sentinel returns [`None`] regardless of registry contents. The nil theme
    /// signals "no theme selected"; loading it would clobber the in-progress draft with a blank.
    #[test]
    fn resolve_autoload_nil_returns_none() {
        let slab = terrain_key(1);
        let t_key = theme_key(1);
        let registry = single_theme_registry(t_key, vec![slab], slab);

        let result = resolve_autoload(ThemeUuid::nil(), &registry);
        assert!(
            result.is_none(),
            "resolve_autoload with the nil sentinel must return None \
             (no theme auto-loaded when nothing is selected)",
        );
    }

    /// C3.5(a) — a non-nil UUID absent from the registry returns [`None`] (the theme was removed
    /// or not yet loaded). The caller must not clobber the draft with a missing def.
    #[test]
    fn resolve_autoload_absent_key_returns_none() {
        let slab = terrain_key(2);
        let registered = theme_key(2);
        let absent = theme_key(99); // intentionally not in the registry
        let registry = single_theme_registry(registered, vec![slab], slab);

        let result = resolve_autoload(absent, &registry);
        assert!(
            result.is_none(),
            "resolve_autoload for a non-nil key absent from the registry must return None",
        );
    }

    /// C3.5(a) — a non-nil UUID present in the registry returns `Some(&def)` for the correct
    /// key. The returned reference is passed straight to `load_theme_into_form` (no double-lookup).
    #[test]
    fn resolve_autoload_present_key_returns_some_def() {
        let slab = terrain_key(3);
        let wall = terrain_key(4);
        let t_key = theme_key(3);
        let registry = single_theme_registry(t_key, vec![slab, wall], slab);

        let result = resolve_autoload(t_key, &registry);
        assert!(
            result.is_some(),
            "resolve_autoload for a non-nil key present in the registry must return Some(&def)",
        );
        let Some(def) = result else {
            // unreachable — the assert above guards this; avoids unwrap/expect per workspace lints
            return;
        };
        assert_eq!(
            def.key, t_key,
            "the returned def's key must equal the looked-up session theme key",
        );
        assert_eq!(
            &**def.display_name, "Hive District",
            "the returned def carries the correct display name from the registry",
        );
    }

    // ── C3.5(b) load_theme_into_form ─────────────────────────────────────────────────────────────

    /// C3.5(b) — [`load_theme_into_form`] replaces the current draft so that key, display name,
    /// terrain palette, and default floor all match the loaded [`UuidThemeDef`]. Verifies the
    /// C3.2 "load-existing" affordance: after the call the form reflects the selected theme's live
    /// definition, not the previous new-theme blank.
    #[test]
    fn load_theme_into_form_replaces_draft_with_def_parts() {
        let slab = terrain_key(5);
        let wall = terrain_key(6);
        let t_key = theme_key(4);

        // Confirm the terrain fixture is sound — the test asserts on the DRAFT after load, not on
        // the registry, but building a real TerrainDefRegistry proves the keys are valid.
        let terrain_reg = TerrainDefRegistry::new([
            (slab, slab_def(slab, "Rockcrete Floor")),
            (wall, wall_def(wall, "Tunnel Wall")),
        ]);
        assert!(
            terrain_reg.def(&slab).is_some(),
            "fixture: slab def must be in the terrain registry"
        );
        assert!(
            terrain_reg.def(&wall).is_some(),
            "fixture: wall def must be in the terrain registry"
        );

        let def = UuidThemeDef {
            key:           t_key,
            display_name:  ThemeDisplayName::new("Underhive Sprawl".to_owned()),
            default_floor: slab,
            terrain:       vec![slab, wall],
        };

        // Start from a blank new-theme draft to prove the fn REPLACES it.
        let mut draft = ThemeDraft::new_theme();
        load_theme_into_form(&mut draft, &def);

        assert_eq!(
            draft.key(),
            t_key,
            "after load_theme_into_form the draft's key must match the def's key (C3.2)",
        );
        assert_eq!(
            draft.display_name(),
            "Underhive Sprawl",
            "after load_theme_into_form the draft's display name must match the def's display name",
        );
        assert_eq!(
            draft.terrain(),
            &[slab, wall],
            "after load_theme_into_form the draft's terrain palette must equal the def's terrain \
             list in the same order (no reordering on load)",
        );
        assert_eq!(
            draft.default_floor(),
            Some(slab),
            "after load_theme_into_form the draft's default floor must match the def's default \
             floor (C3.2 / C6)",
        );
    }

    /// C3.5 round-trip — loads a [`UuidThemeDef`] into the form via [`load_theme_into_form`], then
    /// projects + serializes via [`draft_to_theme_def`] / [`serialize_theme_def`] (the same path
    /// the save button runs — C3.3), then parses the RON back through the GTW-487 loader's parser
    /// (`ron::de::from_str::<UuidThemeDef>`) and asserts structural equality. No magnitudes are
    /// pinned — only that the round-trip is identity (no field dropped or changed).
    #[test]
    fn load_then_save_round_trips_identical() {
        use gdtf_battle_sim::level::UuidThemeDef;

        use crate::theme_form::{draft_to_theme_def, serialize_theme_def};

        let slab = terrain_key(7);
        let wall = terrain_key(8);
        let t_key = theme_key(5);

        let original = UuidThemeDef {
            key:           t_key,
            display_name:  ThemeDisplayName::new("Ash Wastes Outpost".to_owned()),
            default_floor: slab,
            terrain:       vec![slab, wall],
        };

        // Load the def into a fresh draft (the C3.2 form-load path).
        let mut draft = ThemeDraft::new_theme();
        load_theme_into_form(&mut draft, &original);

        // Project + serialize (the C3.3 save path).
        let projected = draft_to_theme_def(&draft, t_key);
        let serialized = serialize_theme_def(&projected);
        assert!(
            serialized.is_ok(),
            "serialize_theme_def must succeed after load_theme_into_form: {:?}",
            serialized.as_ref().err(),
        );
        let Ok(ron_text) = serialized else {
            return;
        };

        // Parse back with the GTW-487 loader's deserializer.
        let reloaded = ron::de::from_str::<UuidThemeDef>(&ron_text);
        assert!(
            reloaded.is_ok(),
            "the serialized def must round-trip through the UuidThemeDef deserializer (the \
             GTW-487 theme loader's parser): {:?}",
            reloaded.as_ref().err(),
        );
        let Ok(reloaded) = reloaded else {
            return;
        };
        assert_eq!(
            reloaded, original,
            "the reloaded UuidThemeDef must be structurally equal to the original def after \
             load → project → serialize → parse (C3.5 round-trip identity)",
        );
    }
}
