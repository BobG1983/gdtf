//! The ARMOR tab's CENTRAL per-body-part piece editor (GTW-479 C1) — one row per
//! [`BodyPart`] (in the canonical `BodyPart::ALL` order): the four clamped stat drags
//! (floor / protection / integrity / hardness) + the closed [`ArmorType`] combo, editing
//! the [`ArmorDraft`]'s pieces through the sim record's own typed fields.

use bevy_egui::egui;
use gdtf_battle_sim::armor::{
    ArmorFloor, ArmorHardness, ArmorIntegrity, ArmorProtection, ArmorType, BodyPart,
};

use crate::armor_form::ArmorDraft;

/// The inclusive range the floor / protection / hardness [`DragValue`](egui::DragValue)s
/// accept. The armor spec documents NO numeric bounds (every magnitude is TBD tuning —
/// `docs/combat/weapons-and-armor.md` §"Open (kept out of this slice)"), so this reuses
/// the editor's ONE documented armor-stat clamp: the TERRAIN form's `ARMOR_RANGE`
/// (`terrain_form_ui/fields.rs`), which clamps the SAME [`ArmorProtection`] /
/// [`ArmorHardness`] stats to `[0, 100]` — no new bound is invented.
const STAT_RANGE: core::ops::RangeInclusive<i32> = 0..=100;

/// The inclusive range the integrity [`DragValue`](egui::DragValue) accepts. Integrity
/// is the suit's durability POOL (it absorbs wear like a structural HP pool), so this
/// reuses the editor's documented pool clamp: the TERRAIN form's `HP_RANGE` ceiling
/// (`terrain_form_ui/fields.rs`, `[0, 1000]`) — a generous ceiling every shipped suit
/// (max 110, `void_hardened.armor.ron`'s torso) fits under, not an invented gameplay
/// bound. `0` is accepted but reads as
/// already-broken armor (the [`ArmorIntegrity`] doc) — the author's explicit choice.
const INTEGRITY_RANGE: core::ops::RangeInclusive<i32> = 0..=1000;

/// The seven [`ArmorType`] wheel nodes in their canonical wheel-node order
/// ([`ArmorType::ALL`]) — the closed option set the per-part type combo offers.
const TYPE_ORDER: [ArmorType; 7] = ArmorType::ALL;

/// The display label for a [`BodyPart`] row — the `docs/combat/resolution.md` §4
/// vocabulary ("Head · Torso · L-Arm · R-Arm · L-Leg · R-Leg").
const fn part_label(part: BodyPart) -> &'static str {
    match part {
        BodyPart::Head => "Head",
        BodyPart::Torso => "Torso",
        BodyPart::LeftArm => "L-Arm",
        BodyPart::RightArm => "R-Arm",
        BodyPart::LeftLeg => "L-Leg",
        BodyPart::RightLeg => "R-Leg",
    }
}

/// The display label for an [`ArmorType`] combo row — the variant's authored RON name
/// (`docs/combat/matchup.md` Table 1 vocabulary).
const fn type_label(armor_type: ArmorType) -> &'static str {
    match armor_type {
        ArmorType::Plated => "Plated",
        ArmorType::Refractive => "Refractive",
        ArmorType::Flak => "Flak",
        ArmorType::Void => "Void",
        ArmorType::Hazard => "Hazard",
        ArmorType::Reinforced => "Reinforced",
        ArmorType::Ceramic => "Ceramic",
    }
}

/// Draw the ARMOR-mode CENTRAL per-body-part editor (GTW-479 C1): a six-column grid —
/// one header row, then one row per [`BodyPart::ALL`] part with the four clamped stat
/// drags + the [`ArmorType`] combo. Each change folds through the matching armor-stat
/// newtype's constructor — the model stays typed end to end (the gang attribute-grid
/// pattern).
pub(crate) fn pieces_panel(ui: &mut egui::Ui, draft: &mut ArmorDraft) {
    ui.heading("Pieces");
    ui.separator();

    egui::Grid::new("armor_pieces_grid")
        .num_columns(6)
        .show(ui, |ui| {
            ui.label("Part");
            ui.label("Floor");
            ui.label("Protection");
            ui.label("Integrity");
            ui.label("Hardness");
            ui.label("Type");
            ui.end_row();

            for part in BodyPart::ALL {
                piece_row(ui, part, draft);
                ui.end_row();
            }
        });
}

/// One body part's stat row: the four clamped drags + the type combo, editing the
/// draft's piece for `part` in place through [`ArmorDraft::piece_mut`].
fn piece_row(ui: &mut egui::Ui, part: BodyPart, draft: &mut ArmorDraft) {
    let piece = draft.piece_mut(part);
    ui.label(part_label(part));
    if let Some(v) = stat_drag(ui, *piece.floor, STAT_RANGE) {
        piece.floor = ArmorFloor::new(v);
    }
    if let Some(v) = stat_drag(ui, *piece.protection, STAT_RANGE) {
        piece.protection = ArmorProtection::new(v);
    }
    if let Some(v) = stat_drag(ui, *piece.integrity, INTEGRITY_RANGE) {
        piece.integrity = ArmorIntegrity::new(v);
    }
    if let Some(v) = stat_drag(ui, *piece.hardness, STAT_RANGE) {
        piece.hardness = ArmorHardness::new(v);
    }
    if let Some(t) = type_combo(ui, part, piece.armor_type) {
        piece.armor_type = t;
    }
}

/// One clamped integer stat [`DragValue`](egui::DragValue) — returns the new value on a
/// change (the drag itself clamps commits into `range`), or [`None`] when untouched.
fn stat_drag(ui: &mut egui::Ui, value: i32, range: core::ops::RangeInclusive<i32>) -> Option<i32> {
    let mut edited = value;
    let changed = ui
        .add(egui::DragValue::new(&mut edited).range(range))
        .changed();
    changed.then_some(edited)
}

/// The per-part [`ArmorType`] [`ComboBox`](egui::ComboBox) — the seven wheel nodes in
/// wheel-node order, the current type as the preview. Returns the newly chosen type, or
/// [`None`] when nothing was clicked. Salted by the part's canonical index so six rows
/// coexist.
fn type_combo(ui: &mut egui::Ui, part: BodyPart, current: ArmorType) -> Option<ArmorType> {
    let mut chosen = None;
    egui::ComboBox::from_id_salt(("armor_piece_type", part.index()))
        .selected_text(type_label(current))
        .show_ui(ui, |ui| {
            for option in TYPE_ORDER {
                if ui
                    .selectable_label(current == option, type_label(option))
                    .clicked()
                {
                    chosen = Some(option);
                }
            }
        });
    chosen
}
