//! The THEME tab's CENTRAL terrain-library multi-select panel (GTW-530 C2) — one
//! `[sprite thumbnail] name [Kind]` row per registered terrain, the tab's primary focus.

use bevy_egui::egui;
use gdtf_battle_presenter::TileRoles;
use gdtf_battle_sim::terrain::def::{TerrainDefRegistry, TerrainUuid};

use crate::{
    egui_shell::sprite_thumb,
    terrain_graphics::terrain_atlas_index,
    theme_form::{ThemeDraft, sim_kind_label},
};

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

// GTW-530 C1: the `.terrain_theme.ron` preview was REMOVED from the THEME tab entirely. Authors do
// not read the raw RON here, so the central space it held is reclaimed for the terrain library
// ([`terrain_library_panel`]) — the tab's primary focus (C2). (This is the deliberate difference
// from the TERRAIN tab's GTW-534, which only DEMOTED its preview to a side strip.)
