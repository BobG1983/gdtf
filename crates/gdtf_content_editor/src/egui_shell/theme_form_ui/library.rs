//! The THEME tab's CENTRAL terrain-library multi-select panel (GTW-530 C2) — one
//! `[sprite thumbnail] name [Kind]` row per registered terrain, the tab's primary focus.

use bevy_egui::egui;
use gdtf_battle_sim::terrain::def::{TerrainDefRegistry, TerrainUuid};
use gdtf_content_families::sprites::SpriteDefRegistry;

use crate::{
    egui_shell::{sprite_thumb, textures::SpriteTextures},
    terrain_graphics::terrain_sprite_def,
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
/// [`terrain_sprite_def`](crate::terrain_graphics::terrain_sprite_def) — the SAME def-driven
/// resolution the
/// battlescape + the TERRAIN picker use (GTW-665), NO hardcoded index — then a selectable
/// multi-select
/// checkbox carrying `"name [Kind]"` (Kind = Wall / Cover / Slab / Emplacement). A check / uncheck
/// routes through
/// [`ThemeDraft::toggle_terrain`], which fail-closes the default floor if the toggled terrain was
/// the chosen floor (the C6 rule) — the multi-select behavior is PRESERVED across the relocation.
///
/// When no registry is present (registries not yet resolved), the area shows a loading marker
/// rather than panicking. A graphic resolving NO sprite def paints the loud magenta missing
/// square (GTW-665 C4); a still-decoding source leaves a fixed-size blank spacer so the row
/// layout stays stable. The sort
/// is rebuilt from the raw `defs()` iterator every draw (deterministic — the registry count / key
/// change drives a natural re-sort). `textures` is the shell's per-path sprite-texture map;
/// `sprites` the GTW-663 sprite-def registry.
pub(crate) fn terrain_library_panel(
    ui: &mut egui::Ui,
    draft: &mut ThemeDraft,
    terrain: Option<&TerrainDefRegistry>,
    sprites: Option<&SpriteDefRegistry>,
    textures: &SpriteTextures,
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
                terrain_library_row(ui, draft, reg, sprites, textures, key, &label);
            }
        });
}

/// Draw ONE terrain-library row — `[sprite thumbnail] name [Kind]` on a single horizontal line
/// (GTW-530 C2).
///
/// The sprite is the terrain's sprite def resolved via
/// [`terrain_sprite_def`](crate::terrain_graphics::terrain_sprite_def) (the presenter's GTW-665
/// resolution — no hardcoded index) and drawn by the shared [`sprite_thumb::draw_thumb`] (which
/// paints the loud magenta missing square for a def-less graphic, or a fixed-size blank spacer
/// while the source decodes, keeping rows aligned
/// — never a panic). The multi-select is a checkbox carrying the `"name [Kind]"` label; a toggle
/// routes through [`ThemeDraft::toggle_terrain`] (fail-closed default floor — C6). When the
/// sprite-def registry is absent the sprite is left
/// unresolved but the checkbox still works.
fn terrain_library_row(
    ui: &mut egui::Ui,
    draft: &mut ThemeDraft,
    reg: &TerrainDefRegistry,
    sprites: Option<&SpriteDefRegistry>,
    textures: &SpriteTextures,
    key: TerrainUuid,
    label: &str,
) {
    ui.horizontal(|ui| {
        // Resolve the sprite the way the presenter + TERRAIN picker do (no hardcoded index): the
        // terrain's graphic name → sprite def (GTW-665).
        let def = sprites.and_then(|sprites| terrain_sprite_def(reg, sprites, &key));
        sprite_thumb::draw_thumb(ui, def, textures);
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
