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
    save_record::LastSaveRecord,
    session::MapEditorSession,
    terrain_form::{ArmorInput, FootfallChoice, HpInput, TerrainDraft, TerrainKindChoice},
};

/// What the terrain form's save button needs beyond the draft it is drawn against.
pub(in crate::egui_shell) struct TerrainSaveContext<'a> {
    /// The session whose theme names the folder the def is written into.
    pub(in crate::egui_shell) session:   &'a MapEditorSession,
    /// The themes registry the session theme's display name is read from.
    pub(in crate::egui_shell) themes:    Option<&'a UuidThemeRegistry>,
    /// Where the outcome of this save is recorded.
    pub(in crate::egui_shell) last_save: &'a mut LastSaveRecord,
}

const HP_RANGE: core::ops::RangeInclusive<u32> = 0..=1000;

const ARMOR_RANGE: core::ops::RangeInclusive<i32> = 0..=100;

const TAG_ORDER: [TerrainTag; 4] = [
    TerrainTag::Openable,
    TerrainTag::BlocksVision,
    TerrainTag::BlocksPathfinding,
    TerrainTag::Indestructible,
];

const UNMINTED_UUID: &str = "(minted on first save)";

const BAND_ORDER: [HeightBand; 3] = [HeightBand::Low, HeightBand::Mid, HeightBand::High];

const fn tag_label(tag: TerrainTag) -> &'static str {
    match tag {
        TerrainTag::Openable => "Openable",
        TerrainTag::BlocksVision => "Blocks Vision",
        TerrainTag::BlocksPathfinding => "Blocks Pathfinding",
        TerrainTag::Indestructible => "Indestructible",
    }
}

const fn band_label(band: HeightBand) -> &'static str {
    match band {
        HeightBand::Low => "Low",
        HeightBand::Mid => "Mid",
        HeightBand::High => "High",
    }
}

pub(in crate::egui_shell) fn field_stack(
    ui: &mut egui::Ui,
    draft: &mut TerrainDraft,
    save: TerrainSaveContext<'_>,
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
    super::entry_sides::entry_sides(ui, draft);
    tag_checkboxes(ui, draft);
    super::blocking::blocking_overrides(ui, draft);
    super::on_death::on_death_form(ui, draft);

    ui.separator();
    uuid_text(ui, draft);

    #[cfg(debug_assertions)]
    {
        ui.separator();
        save_button(ui, draft, save);
    }
    #[cfg(not(debug_assertions))]
    {
        let TerrainSaveContext {
            session,
            themes,
            last_save,
        } = save;
        let _ = (session, themes, last_save);
    }
}

fn name_field(ui: &mut egui::Ui, draft: &mut TerrainDraft) {
    ui.label("Display name");
    let mut name = draft.display_name().to_owned();
    if ui.text_edit_singleline(&mut name).changed() {
        draft.set_display_name(name);
    }
}

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
    let mut names: Vec<&WeaponName> = registry.keys().collect();
    names.sort_by(|a, b| a.as_str().cmp(b.as_str()));

    let selected_text = draft
        .mounted_weapon()
        .map_or_else(|| "(select…)".to_owned(), |weapon| (**weapon).clone());
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

fn tag_checkboxes(ui: &mut egui::Ui, draft: &mut TerrainDraft) {
    ui.label("Tags");
    for tag in TAG_ORDER {
        let mut checked = draft.has_tag(tag);
        if ui.checkbox(&mut checked, tag_label(tag)).changed() {
            draft.toggle_tag(tag);
        }
    }
}

fn uuid_text(ui: &mut egui::Ui, draft: &TerrainDraft) {
    let label = draft
        .uuid()
        .map_or_else(|| UNMINTED_UUID.to_owned(), |uuid| format!("{}", *uuid));
    ui.label(format!("UUID: {label}"));
}

/// `#[cfg(debug_assertions)]`.
#[cfg(debug_assertions)]
fn save_button(ui: &mut egui::Ui, draft: &mut TerrainDraft, save: TerrainSaveContext<'_>) {
    if !ui.button("Save terrain").clicked() {
        return;
    }
    let uuid = draft.ensure_uuid();
    let theme_display = save
        .themes
        .and_then(|themes| themes.def(&save.session.theme()))
        .map_or_else(String::new, |def| (*def.display_name).clone());
    let written = crate::terrain_form::write_terrain(draft, uuid, &theme_display);
    match &written {
        Ok(path) => bevy::log::info!("terrain save: wrote terrain def to `{}`", path.display()),
        Err(err) => bevy::log::error!("terrain save: {err}"),
    }
    save.last_save.record(
        crate::EditorMode::Terrain,
        crate::save_record::SaveOutcome::from_result(written),
    );
}
