use bevy_egui::egui;
use gdtf_battle_sim::{
    armor::{ArmorName, ArmorRegistry},
    ganger::{
        Aim, Cool, GangMember, GangerName, Grit, Luck, Reflexes, Speed, Strength, Toughness,
        derive_stats,
    },
    tuning::GangerStatTuning,
    weapon::{MeleeWeaponRegistry, WeaponName, WeaponRegistry},
};

use crate::gang_form::GangDraft;

const ATTRIBUTE_MIN: f32 = 0.0;
const ATTRIBUTE_MAX: f32 = 100.0;
const SKILL_DECIMALS: usize = 1;
const MELEE_DEFAULT_LABEL: &str = "(default: fists)";

struct LoadoutOptions {
    weapons: Vec<String>,
    melee:   Vec<String>,
    armor:   Vec<String>,
}

pub(crate) fn members_panel(
    ui: &mut egui::Ui,
    draft: &mut GangDraft,
    weapons: Option<&WeaponRegistry>,
    melee: Option<&MeleeWeaponRegistry>,
    armor: Option<&ArmorRegistry>,
    tuning: Option<&GangerStatTuning>,
) {
    ui.heading("Members");
    ui.separator();

    let options = LoadoutOptions {
        weapons: sorted_keys(weapons.map(|r| r.keys().map(|k| k.as_str().to_owned()))),
        melee:   sorted_keys(melee.map(|r| r.keys().map(|k| k.as_str().to_owned()))),
        armor:   sorted_keys(armor.map(|r| r.keys().map(|k| k.as_str().to_owned()))),
    };
    let default_tuning = GangerStatTuning::default();
    let tuning = tuning.unwrap_or(&default_tuning);

    let mut remove: Option<usize> = None;
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for (index, member) in draft.members_mut().iter_mut().enumerate() {
                member_section(ui, index, member, &options, tuning, &mut remove);
            }
        });
    if let Some(index) = remove {
        draft.remove_member(index);
    }
}

fn sorted_keys(keys: Option<impl Iterator<Item = String>>) -> Vec<String> {
    let mut keys: Vec<String> = keys.map(Iterator::collect).unwrap_or_default();
    keys.sort();
    keys
}

fn member_section(
    ui: &mut egui::Ui,
    index: usize,
    member: &mut GangMember,
    options: &LoadoutOptions,
    tuning: &GangerStatTuning,
    remove: &mut Option<usize>,
) {
    egui::CollapsingHeader::new(member.name.as_str())
        .id_salt(("gang_member", index))
        .default_open(true)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label("Name");
                let mut name = member.name.as_str().to_owned();
                if ui.text_edit_singleline(&mut name).changed() {
                    member.name = GangerName::new(name);
                }
                if ui.button("Remove").clicked() {
                    *remove = Some(index);
                }
            });

            loadout_rows(ui, index, member, options);
            attribute_grid(ui, index, member);
            derived_readout(ui, member, tuning);
        });
}

fn loadout_rows(
    ui: &mut egui::Ui,
    index: usize,
    member: &mut GangMember,
    options: &LoadoutOptions,
) {
    if let Some(key) = key_combo(
        ui,
        ("gang_member_weapon", index),
        "Weapon",
        member.weapon.as_str(),
        &options.weapons,
    ) {
        member.weapon = WeaponName::new(key);
    }
    if let Some(choice) = melee_combo(ui, index, member.melee_weapon.as_ref(), &options.melee) {
        member.melee_weapon = match choice {
            MeleeChoice::Default => None,
            MeleeChoice::Key(key) => Some(WeaponName::new(key)),
        };
    }
    if let Some(key) = key_combo(
        ui,
        ("gang_member_armor", index),
        "Armor",
        member.armor.as_str(),
        &options.armor,
    ) {
        member.armor = ArmorName::new(key);
    }
}

fn key_combo(
    ui: &mut egui::Ui,
    id_salt: (&str, usize),
    label: &str,
    current: &str,
    keys: &[String],
) -> Option<String> {
    let mut chosen = None;
    ui.horizontal(|ui| {
        ui.label(label);
        let preview = if current.is_empty() {
            "(select…)"
        } else {
            current
        };
        egui::ComboBox::from_id_salt(id_salt)
            .selected_text(preview)
            .show_ui(ui, |ui| {
                for key in keys {
                    if ui.selectable_label(current == key, key).clicked() {
                        chosen = Some(key.clone());
                    }
                }
            });
    });
    chosen
}

enum MeleeChoice {
    Default,
    Key(String),
}

fn melee_combo(
    ui: &mut egui::Ui,
    index: usize,
    current: Option<&WeaponName>,
    keys: &[String],
) -> Option<MeleeChoice> {
    let mut chosen = None;
    ui.horizontal(|ui| {
        ui.label("Melee");
        let preview = current.map_or(MELEE_DEFAULT_LABEL, |key| key.as_str());
        egui::ComboBox::from_id_salt(("gang_member_melee", index))
            .selected_text(preview)
            .show_ui(ui, |ui| {
                if ui
                    .selectable_label(current.is_none(), MELEE_DEFAULT_LABEL)
                    .clicked()
                {
                    chosen = Some(MeleeChoice::Default);
                }
                for key in keys {
                    let selected = current.is_some_and(|c| c.as_str() == key);
                    if ui.selectable_label(selected, key).clicked() {
                        chosen = Some(MeleeChoice::Key(key.clone()));
                    }
                }
            });
    });
    chosen
}

fn attribute_drag(ui: &mut egui::Ui, label: &str, value: f32) -> Option<f32> {
    ui.label(label);
    let mut edited = value;
    let changed = ui
        .add(
            egui::DragValue::new(&mut edited)
                .speed(0.1)
                .range(ATTRIBUTE_MIN..=ATTRIBUTE_MAX),
        )
        .changed();
    ui.end_row();
    changed.then_some(edited)
}

fn attribute_grid(ui: &mut egui::Ui, index: usize, member: &mut GangMember) {
    egui::Grid::new(("gang_member_attributes", index))
        .num_columns(2)
        .show(ui, |ui| {
            if let Some(v) = attribute_drag(ui, "Speed", *member.speed) {
                member.speed = Speed::new(v);
            }
            if let Some(v) = attribute_drag(ui, "Aim", *member.aim) {
                member.aim = Aim::new(v);
            }
            if let Some(v) = attribute_drag(ui, "Strength", *member.strength) {
                member.strength = Strength::new(v);
            }
            if let Some(v) = attribute_drag(ui, "Toughness", *member.toughness) {
                member.toughness = Toughness::new(v);
            }
            if let Some(v) = attribute_drag(ui, "Reflexes", *member.reflexes) {
                member.reflexes = Reflexes::new(v);
            }
            if let Some(v) = attribute_drag(ui, "Cool", *member.cool) {
                member.cool = Cool::new(v);
            }
            if let Some(v) = attribute_drag(ui, "Grit", *member.grit) {
                member.grit = Grit::new(v);
            }
            if let Some(v) = attribute_drag(ui, "Luck", *member.luck) {
                member.luck = Luck::new(v);
            }
        });
}

fn derived_readout(ui: &mut egui::Ui, member: &GangMember, tuning: &GangerStatTuning) {
    let stats = derive_stats(&member.attributes(), tuning);
    ui.label(format!(
        "Shooting {:.SKILL_DECIMALS$}  Fight {:.SKILL_DECIMALS$}  Reactions \
         {:.SKILL_DECIMALS$}  Morale {:.SKILL_DECIMALS$}",
        *stats.shooting, *stats.fight, *stats.reactions, *stats.morale,
    ));
    ui.label(format!(
        "TU {}  HP {}  Wounds {}  Bottle {}",
        *stats.tu, *stats.hp, *stats.wounds, *stats.bottle,
    ));
}
