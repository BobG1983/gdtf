//! The egui TERRAIN-mode authoring form (GTW-513 C2) — the real terrain form that replaces the
//! C1 stub in the right panel + left palette + central (RON-preview) regions of the egui shell.
//!
//! This module is the egui DRAW + the debug-only save press for the TERRAIN mode. It REUSES the
//! [`terrain_form`](crate::terrain_form) model + save VERBATIM (C2.2): every control reads / writes
//! the state-scoped [`TerrainDraft`] resource through its existing accessors / setters, and the
//! save button calls the existing [`write_terrain`](crate::terrain_form::write_terrain) after the
//! idempotent [`TerrainDraft::ensure_uuid`] mint — round-tripping the GTW-487 loader. The egui draw
//! lives in [`EguiPrimaryContextPass`](bevy_egui::EguiPrimaryContextPass) (bevy-traps #8) via the
//! caller [`editor_egui_ui`](super::shell::editor_egui_ui); the controls mutate the draft in place
//! (no message round-trip — egui is immediate-mode), so the mutations are idempotent under the
//! multipass re-run (bevy-traps #8 fact (b)).
//!
//! ## Layout across the C1 shell panels (C2.1)
//!
//! The form replicates the BEHAVIOR of the GTW-474 `bevy_ui` terrain form (the old LEFT graphic
//! picker / CENTER kind+band controls / RIGHT field stack / STAT live preview), re-pointed onto the
//! C1 shell's panels:
//!
//! - LEFT palette panel — the GRAPHIC-ROLE picker (the 10 [`TerrainGraphicChoice`] role keys), now
//!   a GRID of SPRITE THUMBNAILS (GTW-516) — one clickable sprite per role, the active one
//!   highlighted — resolved via the presenter's [`TileRoles`] vocabulary + the same egui
//!   sprite-render path the GTW-515 prefab palette uses (the shared
//!   [`sprite_thumb`](super::sprite_thumb) helper),
//! - RIGHT mode-form panel — the field stack: display name, the kind segmented row, Max-HP, armor
//!   protection + hardness, the (Wall/Cover-only) height band, the (Slab-only) footfall combo, the
//!   four tag checkboxes, the read-only UUID, and the debug-only Save button,
//! - CENTRAL panel — the live monospace RON preview (re-serialized from the draft each frame).

use bevy_egui::egui;
use gdtf_battle_presenter::TileRoles;
use gdtf_battle_sim::{
    armor::{ArmorHardness, ArmorProtection},
    cover::{CoverHp, HeightBand},
    level::UuidThemeRegistry,
    slab::SlabHp,
    terrain::def::{TerrainTag, TerrainUuid},
};

use crate::{
    egui_shell::sprite_thumb::{self, THUMB_EDGE},
    session::MapEditorSession,
    terrain_form::{
        ArmorInput, FootfallChoice, HpInput, TerrainDraft, TerrainGraphicChoice, TerrainKindChoice,
        draft_to_terrain_def, serialize_terrain_def,
    },
};

/// The inclusive Max-HP range the HP [`DragValue`](egui::DragValue) accepts (C2.1) — a generous
/// ceiling so any authored structural pool fits. Mirrors the old `bevy_ui` form's HP clamp.
const HP_RANGE: core::ops::RangeInclusive<u32> = 0..=1000;

/// The inclusive armor range the protection + hardness [`DragValue`](egui::DragValue)s accept
/// (C2.1). Mirrors the old `bevy_ui` form's armor clamp.
const ARMOR_RANGE: core::ops::RangeInclusive<i32> = 0..=100;

/// The closed tag set the multi-select offers, in display order (C2.1) — the four
/// [`TerrainTag`]s, in the GTW-474 form's order.
const TAG_ORDER: [TerrainTag; 4] = [
    TerrainTag::Openable,
    TerrainTag::BlocksVision,
    TerrainTag::BlocksPathfinding,
    TerrainTag::Indestructible,
];

/// The placeholder UUID line shown before the first save mints a key.
const UNMINTED_UUID: &str = "(minted on first save)";

/// The number of sprite thumbnails per row in the graphic-role picker grid (GTW-516 C1). A
/// framework layout const (a grid column count, not a domain quantity — `no-bare-types` clause-4
/// plumbing carve-out); five keeps the ten roles a tidy two-row grid inside the LEFT palette panel.
const PICKER_COLUMNS: usize = 5;

/// The height-band order the band segmented row renders, left to right.
const BAND_ORDER: [HeightBand; 3] = [HeightBand::Low, HeightBand::Mid, HeightBand::High];

/// The display label for a [`TerrainTag`] checkbox.
const fn tag_label(tag: TerrainTag) -> &'static str {
    match tag {
        TerrainTag::Openable => "Openable",
        TerrainTag::BlocksVision => "Blocks Vision",
        TerrainTag::BlocksPathfinding => "Blocks Pathfinding",
        TerrainTag::Indestructible => "Indestructible",
    }
}

/// The display label for a [`HeightBand`] segment.
const fn band_label(band: HeightBand) -> &'static str {
    match band {
        HeightBand::Low => "Low",
        HeightBand::Mid => "Mid",
        HeightBand::High => "High",
    }
}

/// Draw the TERRAIN-mode GRAPHIC-ROLE picker into the LEFT palette panel (GTW-516 C1/C2) — a GRID
/// of SPRITE THUMBNAILS, one clickable sprite per [`TerrainGraphicChoice`] role, with the draft's
/// active role highlighted, replacing the GTW-513 text role list.
///
/// Each cell shows the ACTUAL sprite for its role: the role's `tile_roles.ron` KEY
/// ([`TerrainGraphicChoice::key`]) is resolved to an atlas index through the presenter's
/// [`TileRoles::index_for_key`] — the SAME resolution the GTW-515 prefab palette uses, drawn via
/// the shared [`sprite_thumb`] helper over the egui-registered terrain sheet (no hardcoded index,
/// no second sprite-loading path — C3). A cell is a clickable [`egui::Button`] that carries the
/// sprite as its image and is `selected` when its role is the draft's active one (C2 highlight); a
/// click writes the choice through [`TerrainDraft::set_graphic`] (C2), which the reactive RON
/// preview then re-serializes. Cells whose role the loaded [`TileRoles`] cannot resolve are shown
/// DISABLED so the picker mirrors the presenter's vocabulary (an absent table — registries not yet
/// resolved — leaves every cell enabled, drawing a labeled placeholder until the sheet loads).
/// Reuses the draft model verbatim (C2.2). `sheet_id` is the terrain sheet's egui texture id
/// (registered by [`load_tile_atlas`](crate::tile_atlas::load_tile_atlas), resolved by the shell).
pub(crate) fn graphic_picker(
    ui: &mut egui::Ui,
    draft: &mut TerrainDraft,
    roles: Option<&TileRoles>,
    sheet_id: Option<egui::TextureId>,
) {
    ui.heading("Graphic role");
    ui.separator();
    let active = draft.graphic();
    // A grid lays the ten roles out as a tidy PICKER_COLUMNS-wide sprite grid inside the LEFT panel
    // space the text list occupied (C1). Track the picked choice and apply it after the grid closure
    // so the mutable draft is not borrowed across the row loop.
    let mut picked: Option<TerrainGraphicChoice> = None;
    egui::Grid::new("terrain_graphic_picker_grid")
        .spacing(egui::vec2(4.0, 4.0))
        .show(ui, |ui| {
            for (slot, choice) in TerrainGraphicChoice::ALL.into_iter().enumerate() {
                if graphic_cell(ui, choice, choice == active, roles, sheet_id) {
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
/// The sprite is the role's atlas index resolved via [`TileRoles::index_for_key`] over
/// [`TerrainGraphicChoice::key`] and built by the shared [`sprite_thumb::thumb_image`] (C3). Until
/// the sheet registers / for an out-of-vocabulary role the button falls back to the role key text,
/// so the cell is never blank. A role the loaded [`TileRoles`] cannot resolve is DISABLED (an
/// absent table leaves every cell enabled so the picker is not inert before load completes).
fn graphic_cell(
    ui: &mut egui::Ui,
    choice: TerrainGraphicChoice,
    selected: bool,
    roles: Option<&TileRoles>,
    sheet_id: Option<egui::TextureId>,
) -> bool {
    // A role the loaded TileRoles can resolve is enabled; an absent table (registries not yet
    // resolved) leaves every cell enabled so the picker is never inert before load completes.
    let resolvable = roles.is_none_or(|roles| roles.index_for_key(choice.key()).is_some());
    let index = roles.and_then(|roles| roles.index_for_key(choice.key()));
    let button = match sprite_thumb::thumb_image(index, sheet_id) {
        // The sprite thumbnail as a selectable, framed image button (the resolved role tile).
        Some(image) => egui::Button::image(image).selected(selected).frame(true),
        // No sprite yet (sheet not registered / out of vocabulary): a labeled placeholder button of
        // the same footprint keeps the grid layout stable until the sheet loads.
        None => egui::Button::new(choice.key())
            .selected(selected)
            .min_size(egui::vec2(THUMB_EDGE, THUMB_EDGE)),
    };
    ui.add_enabled(resolvable, button)
        .on_hover_text(choice.key())
        .clicked()
}

/// Draw the TERRAIN-mode FIELD STACK into the RIGHT mode-form panel (C2.1) — display name, the kind
/// segmented row, Max-HP, armor protection + hardness, the (Wall/Cover-only) height band, the
/// (Slab-only) footfall combo, the four tag checkboxes, the read-only UUID, and the debug-only Save
/// button.
///
/// Every control reads / writes the [`TerrainDraft`] through its existing accessors / setters
/// (C2.2): the kind segmented row routes through [`TerrainDraft::set_kind`] (which fail-closes the
/// footfall on a non-slab kind), Max-HP writes BOTH [`CoverHp`] + [`SlabHp`] through the
/// [`HpInput`] newtype, the armor fields write [`ArmorProtection`] / [`ArmorHardness`] through
/// [`ArmorInput`], the band row is HIDDEN for Slab, and the footfall combo is gated via
/// [`add_enabled`](egui::Ui::add_enabled) on [`TerrainKindChoice::offers_footfall`] (fail-closed —
/// the draft forces `None` off Slab regardless). `session` + `themes` resolve the active theme's
/// display name for the save path.
pub(crate) fn field_stack(
    ui: &mut egui::Ui,
    draft: &mut TerrainDraft,
    session: &MapEditorSession,
    themes: Option<&UuidThemeRegistry>,
) {
    ui.heading("Terrain");
    ui.separator();

    name_field(ui, draft);
    kind_row(ui, draft);
    hp_field(ui, draft);
    armor_fields(ui, draft);
    band_row(ui, draft);
    footfall_combo(ui, draft);
    tag_checkboxes(ui, draft);

    ui.separator();
    uuid_text(ui, draft);

    #[cfg(debug_assertions)]
    {
        ui.separator();
        save_button(ui, draft, session, themes);
    }
    // In a release build the save controls do not compile (the whole save path is debug-only); the
    // bindings are still consumed so the signature is uniform across build profiles.
    #[cfg(not(debug_assertions))]
    {
        let _ = (session, themes);
    }
}

/// The display-name text field — a single-line edit committed straight into the draft (C2.1).
fn name_field(ui: &mut egui::Ui, draft: &mut TerrainDraft) {
    ui.label("Display name");
    let mut name = draft.display_name().to_owned();
    if ui.text_edit_singleline(&mut name).changed() {
        draft.set_display_name(name);
    }
}

/// The kind segmented row — a [`selectable_value`](egui::Ui::selectable_value) per
/// [`TerrainKindChoice`] over the draft's kind (C2.1). Routes through
/// [`TerrainDraft::set_kind`], so switching off Slab fail-closes the footfall to `None`.
fn kind_row(ui: &mut egui::Ui, draft: &mut TerrainDraft) {
    ui.label("Kind");
    ui.horizontal(|ui| {
        let mut kind = draft.kind();
        let before = kind;
        for option in TerrainKindChoice::SEGMENT_ORDER {
            ui.selectable_value(&mut kind, option, option.label());
        }
        if kind != before {
            draft.set_kind(kind);
        }
    });
}

/// The Max-HP [`DragValue`](egui::DragValue) — one field writing BOTH the cover/wall HP pool and the
/// slab HP pool (C2.1; the projection reads the right pool per kind). Edits through the
/// [`HpInput`] newtype clamped to [`HP_RANGE`], then commits into [`CoverHp`] + [`SlabHp`].
fn hp_field(ui: &mut egui::Ui, draft: &mut TerrainDraft) {
    ui.label("Max HP");
    let mut hp = *draft.cover_hp();
    if ui
        .add(egui::DragValue::new(&mut hp).range(HP_RANGE))
        .changed()
    {
        let value = *HpInput::new(hp);
        draft.set_cover_hp(CoverHp::new(value));
        draft.set_slab_hp(SlabHp::new(value));
    }
}

/// The armor protection + hardness [`DragValue`](egui::DragValue)s — both clamped to [`ARMOR_RANGE`]
/// (C2.1), edited through the [`ArmorInput`] newtype, committed into [`ArmorProtection`] /
/// [`ArmorHardness`].
fn armor_fields(ui: &mut egui::Ui, draft: &mut TerrainDraft) {
    ui.label("Armor protection");
    let mut protection = *draft.armor_protection();
    if ui
        .add(egui::DragValue::new(&mut protection).range(ARMOR_RANGE))
        .changed()
    {
        draft.set_armor_protection(ArmorProtection::new(*ArmorInput::new(protection)));
    }

    ui.label("Armor hardness");
    let mut hardness = *draft.armor_hardness();
    if ui
        .add(egui::DragValue::new(&mut hardness).range(ARMOR_RANGE))
        .changed()
    {
        draft.set_armor_hardness(ArmorHardness::new(*ArmorInput::new(hardness)));
    }
}

/// The height-band segmented row — shown ONLY for a Wall / Cover kind (a Slab spans the whole
/// z-boundary, so it carries no band — C2.1). A [`selectable_value`](egui::Ui::selectable_value) per
/// [`HeightBand`] over the draft's band.
fn band_row(ui: &mut egui::Ui, draft: &mut TerrainDraft) {
    if !draft.kind().has_height_band() {
        return;
    }
    ui.label("Height band");
    ui.horizontal(|ui| {
        let mut band = draft.height_band();
        let before = band;
        for option in BAND_ORDER {
            ui.selectable_value(&mut band, option, band_label(option));
        }
        if band != before {
            draft.set_height_band(band);
        }
    });
}

/// The footfall [`ComboBox`](egui::ComboBox) — ENABLED only when the current kind offers footfall
/// (Slab only — C2.1), gated via [`add_enabled_ui`](egui::Ui::add_enabled_ui) on
/// [`TerrainKindChoice::offers_footfall`]. A commit routes through [`TerrainDraft::set_footfall`],
/// which ignores it fail-closed on a non-slab kind.
fn footfall_combo(ui: &mut egui::Ui, draft: &mut TerrainDraft) {
    let offers = draft.kind().offers_footfall();
    ui.label("Footfall (Slab only)");
    ui.add_enabled_ui(offers, |ui| {
        let mut footfall = draft.footfall();
        let before = footfall;
        egui::ComboBox::from_id_salt("terrain_footfall_combo")
            .selected_text(footfall.label())
            .show_ui(ui, |ui| {
                for option in FootfallChoice::ALL {
                    ui.selectable_value(&mut footfall, option, option.label());
                }
            });
        if footfall != before {
            draft.set_footfall(footfall);
        }
    });
}

/// The four tag checkboxes (C2.1) — one per [`TerrainTag`] in [`TAG_ORDER`]; a toggle routes through
/// [`TerrainDraft::toggle_tag`] over the draft's multi-select set.
fn tag_checkboxes(ui: &mut egui::Ui, draft: &mut TerrainDraft) {
    ui.label("Tags");
    for tag in TAG_ORDER {
        let mut checked = draft.has_tag(tag);
        if ui.checkbox(&mut checked, tag_label(tag)).changed() {
            draft.toggle_tag(tag);
        }
    }
}

/// The read-only UUID line (C2.1) — the draft's minted key, or a placeholder before the first save.
fn uuid_text(ui: &mut egui::Ui, draft: &TerrainDraft) {
    let label = draft
        .uuid()
        .map_or_else(|| UNMINTED_UUID.to_owned(), |uuid| format!("{}", *uuid));
    ui.label(format!("UUID: {label}"));
}

/// The debug-only Save button (C2.1) — on press it mints the draft's UUID (idempotent —
/// [`TerrainDraft::ensure_uuid`]), resolves the active theme's display name (the per-theme save
/// dir), and calls the existing [`write_terrain`](crate::terrain_form::write_terrain) (C2.2). On a
/// typed error it logs and writes nothing (never a panic). Debug-only — the whole terrain save path
/// is gated `#[cfg(debug_assertions)]`.
#[cfg(debug_assertions)]
fn save_button(
    ui: &mut egui::Ui,
    draft: &mut TerrainDraft,
    session: &MapEditorSession,
    themes: Option<&UuidThemeRegistry>,
) {
    if !ui.button("Save terrain").clicked() {
        return;
    }
    let uuid = draft.ensure_uuid();
    let theme_display = themes
        .and_then(|themes| themes.def(&session.theme()))
        .map_or_else(String::new, |def| (*def.display_name).clone());
    match crate::terrain_form::write_terrain(draft, uuid, &theme_display) {
        Ok(path) => bevy::log::info!("terrain save: wrote terrain def to `{}`", path.display()),
        Err(err) => bevy::log::error!("terrain save: {err}"),
    }
}

/// Draw the live monospace `.terrain_def.ron` PREVIEW into the CENTRAL panel (C2.1) — a
/// [`ScrollArea`](egui::ScrollArea) of [`ui.monospace`](egui::Ui::monospace) text, re-serialized
/// from the draft each frame so it tracks every edit.
///
/// Projects the draft with its minted key if present, else the [`TerrainUuid::nil`] sentinel as a
/// placeholder (the real key is minted on save) — exactly the old `bevy_ui` preview's behavior.
/// Reuses [`draft_to_terrain_def`] + [`serialize_terrain_def`] verbatim (C2.2); a serialize error
/// renders as an inline marker rather than a panic.
pub(crate) fn ron_preview(ui: &mut egui::Ui, draft: &TerrainDraft) {
    ui.heading("Preview (.terrain_def.ron)");
    ui.separator();
    let key = draft.uuid().unwrap_or_else(TerrainUuid::nil);
    let def = draft_to_terrain_def(draft, key);
    let body =
        serialize_terrain_def(&def).unwrap_or_else(|err| format!("<serialize error: {err}>"));
    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.monospace(body);
    });
}
