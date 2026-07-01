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
//! ## Layout across the C1 shell panels (C3.1)
//!
//! - LEFT palette panel — the resolved-stats readout for the current default-floor terrain (or a
//!   placeholder when none is selected), reusing [`resolved_stats`](crate::theme_form::resolved_stats)
//!   (the C3 stat proof) — a text summary + an [`egui::ProgressBar`] HP bar.
//! - RIGHT mode-form panel — the field stack: display name, the terrain-UUID library multi-select
//!   (a [`ScrollArea`](egui::ScrollArea) of checkboxes from [`TerrainDefRegistry`], sorted by
//!   name, each `"name [Kind]"`), the default-floor [`ComboBox`] (Slab-first via
//!   [`floor_candidates`](crate::theme_form::floor_candidates)), the read-only UUID, the New-theme
//!   button, and the debug-only Save button.
//! - CENTRAL panel — the live monospace RON preview (re-serialized on change).
//!
//! ## Load-existing (C3.2)
//!
//! When the global theme `ComboBox` in the top bar selects a theme while in THEME mode, the shell
//! calls [`load_theme_into_form`] so the draft shows the selected theme's current definition. On
//! entering THEME mode with a theme already selected, the shell does the same check.

use bevy_egui::egui;
use gdtf_battle_sim::{
    level::{ThemeUuid, UuidThemeDef, UuidThemeRegistry},
    terrain::def::{TerrainDefRegistry, TerrainSimKind, TerrainUuid},
};

use crate::theme_form::{
    ThemeDraft, draft_to_theme_def, floor_candidates, resolved_stats, serialize_theme_def,
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

/// Draw the THEME-mode FIELD STACK into the RIGHT mode-form panel (C3.1) — display name, the
/// terrain library multi-select (a [`ScrollArea`] of checkboxes from the [`TerrainDefRegistry`],
/// sorted by name, each `"name [Kind]"`), the default-floor [`ComboBox`] (Slab-kind FIRST via
/// [`floor_candidates`]), the read-only UUID key, the New-theme button (mints a fresh draft), and
/// the debug-only Save button.
///
/// Every control reads / writes the [`ThemeDraft`] through its existing accessors / setters
/// (C3.3): the terrain checkboxes route through [`ThemeDraft::toggle_terrain`] (which fail-closes
/// the default floor when a terrain is removed — the C6 rule), the floor combo routes through
/// [`ThemeDraft::set_default_floor`] (which ignores a key not in the palette — fail-closed C6).
pub(crate) fn field_stack(
    ui: &mut egui::Ui,
    draft: &mut ThemeDraft,
    terrain: Option<&TerrainDefRegistry>,
) {
    ui.heading("Theme");
    ui.separator();

    name_field(ui, draft);
    terrain_library(ui, draft, terrain);
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

/// The terrain library multi-select (C3.1) — a [`ScrollArea`] of checkboxes, one per registered
/// terrain, sorted by display name so the order is deterministic (the registry is a `HashMap`).
/// Each row's label is `"name [Kind]"` where Kind is Wall / Cover / Slab. A check / uncheck
/// routes through [`ThemeDraft::toggle_terrain`], which fail-closes the default floor if the
/// toggled terrain was the chosen floor (the C6 rule).
///
/// When no registry is present (registries not yet resolved), the area is empty rather than
/// panicking. The sort is rebuilt from the raw `defs()` iterator every draw (deterministic — the
/// registry count / key change drives a natural re-sort, matching the ticket's rebuild-on-change
/// requirement).
fn terrain_library(
    ui: &mut egui::Ui,
    draft: &mut ThemeDraft,
    terrain: Option<&TerrainDefRegistry>,
) {
    ui.label("Terrain library");
    egui::ScrollArea::vertical()
        .id_salt("theme_terrain_library")
        .max_height(200.0)
        .show(ui, |ui| {
            let Some(reg) = terrain else {
                ui.label("(loading…)");
                return;
            };
            // Sort by display name for a deterministic, reader-friendly list.
            let mut entries: Vec<(TerrainUuid, String)> = reg
                .defs()
                .map(|(key, def)| {
                    let label =
                        format!("{}  [{}]", *def.display_name, sim_kind_label(&def.sim_kind));
                    (*key, label)
                })
                .collect();
            entries.sort_by(|(_, a), (_, b)| a.cmp(b));

            for (key, label) in entries {
                let mut checked = draft.has_terrain(key);
                if ui.checkbox(&mut checked, &label).changed() {
                    draft.toggle_terrain(key);
                }
            }
        });
}

/// The default-floor [`ComboBox`] (C3.1) — offers the draft's own terrain as candidates (via
/// [`floor_candidates`], Slab-kind FIRST), with the current default floor pre-selected. Choosing
/// a row routes through [`ThemeDraft::set_default_floor`], which ignores it unless the terrain is
/// in the palette (fail-closed C6). When the palette is empty the combo shows `"(none)"` and is
/// disabled. Requires the terrain registry to resolve display names; falls back to a read-only
/// label when absent.
fn floor_combo(ui: &mut egui::Ui, draft: &mut ThemeDraft, terrain: Option<&TerrainDefRegistry>) {
    ui.label("Default floor");
    let Some(reg) = terrain else {
        ui.label("(loading…)");
        return;
    };
    let candidates = floor_candidates(draft, reg);
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

/// Draw the live monospace `.terrain_theme.ron` PREVIEW into the CENTRAL panel (C3.1) — a
/// [`ScrollArea`](egui::ScrollArea) of [`ui.monospace`](egui::Ui::monospace) text, re-serialized
/// from the draft each frame so it tracks every edit.
///
/// Projects the draft with its current key via [`draft_to_theme_def`] + [`serialize_theme_def`]
/// verbatim (C3.3); a serialize error renders as an inline marker rather than a panic. The default
/// floor uses the [`TerrainUuid::nil`] sentinel if none is chosen yet (a defensible preview
/// placeholder — the C6 validation rejects it on save).
pub(crate) fn ron_preview(ui: &mut egui::Ui, draft: &ThemeDraft) {
    ui.heading("Preview (.terrain_theme.ron)");
    ui.separator();
    let key = draft.key();
    let def = draft_to_theme_def(draft, key);
    let body = serialize_theme_def(&def).unwrap_or_else(|err| format!("<serialize error: {err}>"));
    egui::ScrollArea::vertical()
        .id_salt("theme_ron_preview")
        .show(ui, |ui| {
            ui.monospace(body);
        });
}

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

/// The short human label for a terrain sim kind (Wall / Cover / Slab) — shown beside each library
/// row's name so the author sees the structural kind at a glance (C3.1). Mirrors the private
/// [`sim_kind_label`](crate::theme_form::resolve::sim_kind_label) in `resolve.rs` (used here
/// in the library renderer; this copy avoids reaching into the private submodule).
const fn sim_kind_label(kind: &TerrainSimKind) -> &'static str {
    match kind {
        TerrainSimKind::Wall { .. } => "Wall",
        TerrainSimKind::Cover { .. } => "Cover",
        TerrainSimKind::Slab { .. } => "Slab",
    }
}

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
