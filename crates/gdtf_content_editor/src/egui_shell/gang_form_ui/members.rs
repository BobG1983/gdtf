//! The GANG tab's CENTRAL member-list editor (GTW-636 C1) — one collapsible section per
//! roster member: the name field, the weapon / melee / armor registry dropdowns, the
//! eight base-attribute drags, the LIVE derived-stats readout (the real GTW-384
//! pipeline, never a reimplementation), and the remove button.

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

/// The base-attribute drag clamp floor — the retired in-game editor's exact range,
/// kept for parity (its numeric fields clamped commits to `[0, 100]`).
const ATTRIBUTE_MIN: f32 = 0.0;
/// The base-attribute drag clamp ceiling (see [`ATTRIBUTE_MIN`]).
const ATTRIBUTE_MAX: f32 = 100.0;
/// How many fractional digits the f32 SKILL stats render with (the retired editor's
/// derived-stat formatting: skills at one decimal, integer pools bare).
const SKILL_DECIMALS: usize = 1;
/// The melee dropdown's None row — an unauthored melee key resolves to the shipped
/// `fists` default at battle setup (the GTW-37 D3 ruling), so the row names that.
const MELEE_DEFAULT_LABEL: &str = "(default: fists)";

/// The sorted registry key lists the loadout dropdowns offer — bundled so the
/// per-member section keeps a legible signature. Each list is the registry's keys
/// sorted by string (registry iteration order is unspecified); an unresolved registry
/// yields an empty list (the dropdown then offers nothing — degraded, never a panic).
struct LoadoutOptions {
    /// Sorted ranged-weapon keys ([`WeaponRegistry`]).
    weapons: Vec<String>,
    /// Sorted melee-weapon keys ([`MeleeWeaponRegistry`]).
    melee:   Vec<String>,
    /// Sorted armor keys ([`ArmorRegistry`]).
    armor:   Vec<String>,
}

/// Draw the GANG-mode CENTRAL member list (GTW-636 C1): a scrollable stack of
/// per-member editors over the [`GangDraft`]'s sim records. A remove press is folded
/// into the draft AFTER the loop (one structural edit per frame — indices stay honest;
/// idempotent under the egui multipass re-run because a click is a discrete event).
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
    // The GTW-384 derivation weights; the editor loads no stat tuning, so the
    // const-default weights back the readout (the retired editor's exact fallback).
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

/// Collect an optional key iterator into a sorted list (empty when the registry has
/// not resolved yet).
fn sorted_keys(keys: Option<impl Iterator<Item = String>>) -> Vec<String> {
    let mut keys: Vec<String> = keys.map(Iterator::collect).unwrap_or_default();
    keys.sort();
    keys
}

/// One member's collapsible editor: name + remove, the three loadout dropdowns, the
/// eight attribute drags, and the derived readout. The section id is salted by INDEX
/// (never the name) so renaming keeps the open/closed state; `default_open` so a
/// loaded gang's values are immediately visible.
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

/// The three loadout dropdowns — weapon / melee / armor keys straight from the sorted
/// registry lists (the key IS the display label: specs carry no display name).
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

/// A registry-key [`ComboBox`](egui::ComboBox) row: the current key as the preview
/// (`"(select…)"` when empty), one selectable row per sorted key. Returns the newly
/// chosen key, or [`None`] when nothing was clicked.
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

/// What the author clicked in the MELEE dropdown — a named choice (no-bare-types: an
/// `Option<Option<String>>` return would bury "cleared back to the fists default" in
/// nesting): [`Default`](MeleeChoice::Default) clears the authored key (the model's
/// `None` — resolves to `fists` at setup), [`Key`](MeleeChoice::Key) authors one.
enum MeleeChoice {
    /// Clear the authored key — back to the shipped `fists` default.
    Default,
    /// Author this melee-weapon registry key.
    Key(String),
}

/// The MELEE dropdown — like [`key_combo`] plus a leading [`MELEE_DEFAULT_LABEL`] row
/// mapping to [`MeleeChoice::Default`]. Returns the clicked choice, or [`None`] when
/// nothing was clicked.
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

/// One attribute drag row inside the grid — returns the new value on a change, clamped
/// by the drag itself to the parity range.
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

/// The eight base-attribute drags (a two-column grid). Each change folds through the
/// matching attribute newtype's constructor — the model stays typed end to end.
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

/// The LIVE derived-stats readout — the REAL GTW-384
/// [`derive_stats`] pipeline over the member's current attributes × the tuning weights
/// (recomputed every pass, so a drag updates it the same frame — the retired editor's
/// C3 "displayed == pipeline(attrs)" contract). Skills render at [`SKILL_DECIMALS`]
/// decimals, the integer pools bare.
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
