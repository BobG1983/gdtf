//! The TERRAIN tab's STAT FIELD STACK (GTW-513 C2.1) — every draft control (name, kind,
//! HP, armor, band, footfall, the GTW-574 Emplacement mounted-weapon combo, tags, UUID)
//! plus the debug-only save press.

use bevy_egui::egui;
use gdtf_battle_sim::{
    armor::{ArmorHardness, ArmorProtection},
    cover::{CoverHp, HeightBand},
    level::UuidThemeRegistry,
    slab::SlabHp,
    terrain::def::TerrainTag,
    weapon::{WeaponName, WeaponRegistry},
};

use crate::{
    session::MapEditorSession,
    terrain_form::{ArmorInput, FootfallChoice, HpInput, TerrainDraft, TerrainKindChoice},
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

/// Draw the TERRAIN-mode FIELD STACK (C2.1) — display name, the kind segmented row, Max-HP, armor
/// protection + hardness, the (Wall/Cover/Emplacement) height band, the (Slab-only) footfall
/// combo, the (Emplacement-only) mounted-weapon combo (GTW-574 C5), the four tag checkboxes, the
/// read-only UUID, and the debug-only Save button. Hosted in the CENTRAL primary region by
/// [`primary_panel`](super::panel::primary_panel) since GTW-534 C1 (formerly the RIGHT mode-form
/// panel).
///
/// Every control reads / writes the [`TerrainDraft`] through its existing accessors / setters
/// (C2.2): the kind segmented row routes through [`TerrainDraft::set_kind`] (which fail-closes the
/// footfall on a non-slab kind AND the mounted weapon on a non-Emplacement kind — GTW-574), Max-HP
/// writes BOTH [`CoverHp`] + [`SlabHp`] through the [`HpInput`] newtype, the armor fields write
/// [`ArmorProtection`] / [`ArmorHardness`] through [`ArmorInput`], the band row is HIDDEN for
/// Slab, the footfall combo is gated via [`add_enabled_ui`](egui::Ui::add_enabled_ui) on
/// [`TerrainKindChoice::offers_footfall`], and the mounted-weapon combo is DRAWN only for the
/// Emplacement kind, its options the live [`WeaponRegistry`] keys sorted for a stable order (Q1 —
/// never free-text). `session` + `themes` resolve the active theme's display name for the save
/// path.
pub(crate) fn field_stack(
    ui: &mut egui::Ui,
    draft: &mut TerrainDraft,
    session: &MapEditorSession,
    themes: Option<&UuidThemeRegistry>,
    weapons: Option<&WeaponRegistry>,
) {
    ui.heading("Terrain");
    ui.separator();

    name_field(ui, draft);
    kind_row(ui, draft);
    hp_field(ui, draft);
    armor_fields(ui, draft);
    band_row(ui, draft);
    footfall_combo(ui, draft);
    mounted_weapon_combo(ui, draft, weapons);
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
/// [`TerrainDraft::set_kind`], so switching off Slab fail-closes the footfall to `None` and
/// switching off Emplacement clears the mounted weapon (GTW-574 C5). The segment set is
/// [`TerrainKindChoice::SEGMENT_ORDER`] — compiler-tied to the canonical kind inventory
/// (GTW-574 C4), so a new terrain kind appears here without an editor edit.
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

/// The height-band segmented row — shown ONLY for a kind that carries a band (Wall / Cover /
/// Emplacement per [`TerrainKindChoice::has_height_band`]; a Slab spans the whole z-boundary, so
/// it carries no band — C2.1). A [`selectable_value`](egui::Ui::selectable_value) per
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

/// The mounted-weapon [`ComboBox`](egui::ComboBox) (GTW-574 C5, Q1) — drawn ONLY for the
/// Emplacement kind. Its options are the live [`WeaponRegistry`] keys SORTED by their
/// string for a stable order (the registry is a hash map; the editor already resolves +
/// hot-redrives it), each a [`selectable_label`](egui::Ui::selectable_label); a click
/// commits the picked [`WeaponName`] through [`TerrainDraft::set_mounted_weapon`] (the
/// Emplacement-gated fail-closed setter). NEVER free-text — an unselectable weapon
/// cannot be authored, and an Emplacement with no selection fails the save closed (C6).
/// Before the registry resolves, a loading marker keeps the row stable (never a panic).
fn mounted_weapon_combo(
    ui: &mut egui::Ui,
    draft: &mut TerrainDraft,
    weapons: Option<&WeaponRegistry>,
) {
    if draft.kind() != TerrainKindChoice::Emplacement {
        return;
    }
    ui.label("Mounted weapon");
    let Some(registry) = weapons else {
        ui.label("(loading…)");
        return;
    };
    // Sort the registry keys for a stable, reader-friendly option order (the registry is a
    // HashMap — unsorted iteration order would jitter across frames).
    let mut names: Vec<&WeaponName> = registry.keys().collect();
    names.sort_by(|a, b| a.as_str().cmp(b.as_str()));

    let selected_text = draft
        .mounted_weapon()
        .map_or_else(|| "(select…)".to_owned(), |weapon| (**weapon).clone());
    // Track the pick and apply it after the combo closure so the mutable draft is not
    // borrowed across the option loop (the graphic-picker pattern).
    let mut picked: Option<WeaponName> = None;
    egui::ComboBox::from_id_salt("terrain_mounted_weapon_combo")
        .selected_text(selected_text)
        .show_ui(ui, |ui| {
            for name in names {
                let is_selected = draft.mounted_weapon() == Some(name);
                if ui.selectable_label(is_selected, name.as_str()).clicked() {
                    picked = Some(name.clone());
                }
            }
        });
    if let Some(name) = picked {
        draft.set_mounted_weapon(Some(name));
    }
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
/// typed error — including the GTW-574 fail-closed
/// [`MissingMountedWeapon`](crate::terrain_form::SaveTerrainError::MissingMountedWeapon) — it logs
/// and writes nothing (never a panic). Debug-only — the whole terrain save path is gated
/// `#[cfg(debug_assertions)]`.
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
