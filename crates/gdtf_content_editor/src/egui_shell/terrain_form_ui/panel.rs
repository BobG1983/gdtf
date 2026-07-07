//! The TERRAIN tab's CENTRAL primary region (GTW-534 C1) — the two-column composition
//! ([`primary_panel`]) and the sprite-grid graphic picker (GTW-516).

use bevy_egui::egui;
use gdtf_battle_presenter::{TileRole, resolve_sprite};
use gdtf_battle_sim::{level::UuidThemeRegistry, weapon::WeaponRegistry};
use gdtf_content_families::sprites::SpriteDefRegistry;

use super::fields::field_stack;
use crate::{
    egui_shell::{
        sprite_thumb::{self, THUMB_EDGE},
        textures::SpriteTextures,
    },
    session::MapEditorSession,
    terrain_form::{TerrainDraft, offered_graphic_roles},
};

/// The number of sprite thumbnails per row in the graphic-role picker grid (GTW-516 C1). A
/// framework layout const (a grid column count, not a domain quantity — `no-bare-types` clause-4
/// plumbing carve-out); five keeps the fifteen def-authorable roles (GTW-566 C5) a tidy
/// three-row grid inside the panel.
const PICKER_COLUMNS: usize = 5;

/// Draw the TERRAIN-mode CENTRAL PRIMARY region (GTW-534 C1) — the two authoring foci, the sprite
/// picker + the stat field stack, side by side in the largest / most-prominent panel.
///
/// This is the emphasis rework: rather than the `.terrain_def.ron` preview dominating the central
/// region (the GTW-513 layout), the central region now hosts what authoring a terrain is actually
/// about — SETTING STATS ([`field_stack`]) and PICKING A SPRITE (the GTW-516 [`graphic_picker`]).
/// The preview is demoted to the LEFT secondary strip by the shell (C2). This fn is pure
/// COMPOSITION: it lays the two existing draw fns into two columns (picker left, fields right) and
/// does not reimplement either — every stat edit / sprite selection still routes through the same
/// [`TerrainDraft`] setters, so nothing is broken by the relocation (C3). The columns split the
/// central width evenly so both foci get generous, co-visible space. `weapons` is the live
/// [`WeaponRegistry`] the Emplacement-only mounted-weapon combo offers (GTW-574 C5).
pub(crate) fn primary_panel(
    ui: &mut egui::Ui,
    draft: &mut TerrainDraft,
    session: &MapEditorSession,
    themes: Option<&UuidThemeRegistry>,
    sprites: Option<&SpriteDefRegistry>,
    weapons: Option<&WeaponRegistry>,
    textures: &SpriteTextures,
) {
    // Two evenly-split columns: the sprite-grid picker on the left, the stat field stack on the
    // right. `columns` gives each a `&mut Ui`; the borrow checker requires we not borrow `draft`
    // across both closures, and `columns` runs them sequentially, so passing `&mut *draft` into each
    // in turn is sound (they do not overlap in time).
    ui.columns(2, |columns| {
        if let [picker_col, fields_col] = columns {
            graphic_picker(picker_col, draft, sprites, textures);
            field_stack(fields_col, draft, session, themes, weapons);
        }
    });
}

/// Draw the TERRAIN-mode GRAPHIC-ROLE picker (GTW-516 C1/C2) — a GRID of SPRITE THUMBNAILS, one
/// clickable sprite per OFFERED [`TileRole`], with the draft's active role highlighted,
/// replacing the GTW-513 text role list. Hosted in the CENTRAL primary region by [`primary_panel`]
/// since GTW-534 C1 (formerly the LEFT palette panel).
///
/// The offered set is [`offered_graphic_roles`] — the presenter's [`TileRole`] vocabulary
/// filtered by [`TileRole::def_authorable`] (GTW-566 C5), so the GTW-543 emplacement and the four
/// GTW-470 oriented stairs are authorable here with no hand-mirrored editor role list. Each cell
/// shows the ACTUAL sprite for its role: the role key resolves through the presenter's
/// [`resolve_sprite`] over the GTW-663 [`SpriteDefRegistry`] — the SAME def-driven resolution the
/// battlescape draws with (GTW-665) — drawn via the shared [`sprite_thumb`] helper over the
/// shell-resolved per-path textures (no hardcoded index, no second sprite-loading path — C3). A
/// cell is a clickable [`egui::Button`] that carries the sprite as its image and is `selected`
/// when its role is the draft's active one (C2 highlight); a click writes the role through
/// [`TerrainDraft::set_graphic`] (C2), which the reactive RON preview then re-serializes. Before
/// a role's sprite def / source texture resolves, a labeled placeholder keeps the grid
/// stable. Reuses the draft model verbatim (C2.2).
fn graphic_picker(
    ui: &mut egui::Ui,
    draft: &mut TerrainDraft,
    sprites: Option<&SpriteDefRegistry>,
    textures: &SpriteTextures,
) {
    ui.heading("Graphic role");
    ui.separator();
    let active = draft.graphic();
    // A grid lays the offered roles out as a tidy PICKER_COLUMNS-wide sprite grid (C1). Track the
    // picked role and apply it after the grid closure so the mutable draft is not borrowed across
    // the row loop.
    let mut picked: Option<TileRole> = None;
    egui::Grid::new("terrain_graphic_picker_grid")
        .spacing(egui::vec2(4.0, 4.0))
        .show(ui, |ui| {
            for (slot, choice) in offered_graphic_roles().into_iter().enumerate() {
                if graphic_cell(ui, choice, choice == active, sprites, textures) {
                    picked = Some(choice);
                }
                if (slot + 1) % PICKER_COLUMNS == 0 {
                    ui.end_row();
                }
            }
        });
    if let Some(choice) = picked {
        draft.set_graphic(choice);
    }
}

/// Draw ONE graphic-role cell — a clickable sprite [`egui::Button`], `selected` (highlighted) when
/// `selected`, returning whether it was clicked (GTW-516 C1/C2).
///
/// The sprite is the role key's sprite def resolved via the presenter's [`resolve_sprite`]
/// (GTW-665 — the ONE resolution) and built by the shared
/// [`sprite_thumb::thumb_image`] (C3). Until the def's source texture registers (or the
/// registry is still loading) the button falls back to the role key text, so the cell is
/// never blank.
fn graphic_cell(
    ui: &mut egui::Ui,
    choice: TileRole,
    selected: bool,
    sprites: Option<&SpriteDefRegistry>,
    textures: &SpriteTextures,
) -> bool {
    let def = sprites.and_then(|sprites| resolve_sprite(sprites, choice.as_key()));
    let button = match sprite_thumb::thumb_image(def, textures) {
        // The sprite thumbnail as a selectable, framed image button (the resolved role tile).
        Some(image) => egui::Button::image(image).selected(selected).frame(true),
        // No sprite yet (def / source texture still loading): a labeled placeholder
        // button of the same footprint keeps the grid layout stable until it resolves.
        None => egui::Button::new(choice.as_key())
            .selected(selected)
            .min_size(egui::vec2(THUMB_EDGE, THUMB_EDGE)),
    };
    ui.add(button).on_hover_text(choice.as_key()).clicked()
}
