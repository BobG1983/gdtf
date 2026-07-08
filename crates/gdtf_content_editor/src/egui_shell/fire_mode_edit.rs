//! The REUSABLE **fire-mode row editor** (GTW-669 C2) — the egui controls that edit one
//! [`FireModeSpec`] in place: the closed [`ModeKind`] combo plus the per-mode numbers
//! (`cone_mult` / `tu_percent` / `shots`) and the [`HitType`] template row.
//!
//! A SHARED `egui_shell` sibling (not an `attachment_form_ui` private) on purpose: the
//! ATTACHMENT mode's `GainFireMode` effect row edits exactly one authored fire mode, and
//! the GTW-670 WEAPON form's `fire_mode:` list editor edits the same row shape — both
//! consume THIS widget, so the fire-mode authoring surface has one definition (the
//! `theme_combo` / `sprite_thumb` shared-leaf precedent). The [`HitType`] half is also
//! exposed on its own ([`hit_type_row`] — extended IN PLACE for GTW-670, not forked):
//! the WEAPON form's `on_death: Explode` payload authors a bare [`HitType`] outside any
//! fire mode, and it draws the SAME template combo + payload drags.
//!
//! Every commit folds through the sim newtypes' constructors, and the drag clamps are
//! the payload TYPES' own ranges — the specs document NO tighter numeric bounds (the
//! GTW-479 no-invented-clamps ruling), only non-negativity by construction for the
//! unsigned counts.

use bevy_egui::egui;
use gdtf_battle_sim::weapon::{
    AoeRange, BlastRadius, ConeHalfAngle, FireModeSpec, HitType, ModeConeMult, ModeKind, ModeShots,
    ModeTuPercent,
};

/// The three closed [`ModeKind`]s the kind combo offers, in the sim's declaration order.
const MODE_KINDS: [ModeKind; 3] = [ModeKind::Single, ModeKind::Burst, ModeKind::Full];

/// The seed each [`HitType`] KIND switches to when the row's template combo picks it —
/// the payloads are the sim doc examples (`Blast(radius: 2)` / `Cone(range: 3, angle:
/// 30.0)` — the `fire_mode` module's own authored-RON illustrations; `Line` reuses the
/// cone's documented depth), starting points the author immediately re-tunes, never
/// shipped pins. [`HitType::Single`] is the sim's documented default.
const HIT_TYPE_TEMPLATES: [HitType; 4] = [
    HitType::Single,
    HitType::Blast {
        radius: BlastRadius::new(2),
    },
    HitType::Cone {
        range: AoeRange::new(3),
        angle: ConeHalfAngle::new(30.0),
    },
    HitType::Line {
        range: AoeRange::new(3),
    },
];

/// The display label for a [`HitType`] combo row — the authored RON variant name (a
/// payload-carrying variant labels by its KIND, so the combo stays a template picker).
/// Takes the 8-byte `Copy` value by value (clippy `trivially_copy_pass_by_ref`).
const fn hit_type_label(hit_type: HitType) -> &'static str {
    match hit_type {
        HitType::Single => "Single",
        HitType::Blast { .. } => "Blast",
        HitType::Cone { .. } => "Cone",
        HitType::Line { .. } => "Line",
    }
}

/// Draw the fire-mode row editor over `spec`, editing it IN PLACE: the [`ModeKind`]
/// combo (the closed `Single` / `Burst` / `Full` vocabulary — the authored RON variant
/// names), drags for the three per-mode numbers, and the [`HitType`] template row.
/// Salted by `index` (the row's position in the calling list) so several rows coexist —
/// the injury effects-list convention.
pub(crate) fn fire_mode_row(ui: &mut egui::Ui, index: usize, spec: &mut FireModeSpec) {
    ui.horizontal(|ui| {
        ui.label("Kind");
        egui::ComboBox::from_id_salt(("fire_mode_kind", index))
            .selected_text(format!("{:?}", spec.kind))
            .show_ui(ui, |ui| {
                for option in MODE_KINDS {
                    ui.selectable_value(&mut spec.kind, option, format!("{option:?}"));
                }
            });
        mode_number_drags(ui, spec);
    });
    hit_type_row(ui, ("fire_mode_hit_type", index), &mut spec.hit_type);
}

/// Draw the [`HitType`] row on its own — the template combo plus that kind's payload
/// drags, editing the value IN PLACE (GTW-670: the WEAPON form's `on_death: Explode`
/// payload authors a bare [`HitType`], so the row is exposed beside
/// [`fire_mode_row`], which delegates to it). `salt` disambiguates coexisting rows
/// (the calling list supplies its own id source).
pub(crate) fn hit_type_row(
    ui: &mut egui::Ui,
    salt: impl std::hash::Hash + std::fmt::Debug,
    hit_type: &mut HitType,
) {
    ui.horizontal(|ui| {
        hit_type_combo(ui, salt, hit_type);
        hit_type_payload(ui, hit_type);
    });
}

/// The three per-mode number drags (`cone_mult` / `tu_percent` / `shots`), each folding
/// through its sim newtype's constructor. The floats carry NO documented bounds (the
/// specs' magnitudes are tuning-open), so no clamp is invented; the shot count is the
/// `u16` type's own range.
fn mode_number_drags(ui: &mut egui::Ui, spec: &mut FireModeSpec) {
    ui.label("×cone");
    let mut cone = *spec.cone_mult;
    if ui
        .add(egui::DragValue::new(&mut cone).speed(0.05))
        .changed()
    {
        spec.cone_mult = ModeConeMult::new(cone);
    }
    ui.label("TU%");
    let mut tu = *spec.tu_percent;
    if ui.add(egui::DragValue::new(&mut tu).speed(0.01)).changed() {
        spec.tu_percent = ModeTuPercent::new(tu);
    }
    ui.label("shots");
    let mut shots = *spec.shots;
    if ui.add(egui::DragValue::new(&mut shots)).changed() {
        spec.shots = ModeShots::new(shots);
    }
}

/// The [`HitType`] template combo — picking a DIFFERENT kind replaces the value with
/// that kind's [`HIT_TYPE_TEMPLATES`] seed (the current kind's row is a no-op, so an
/// open combo never wipes a tuned payload — the injury variant-combo convention).
fn hit_type_combo(
    ui: &mut egui::Ui,
    salt: impl std::hash::Hash + std::fmt::Debug,
    hit_type: &mut HitType,
) {
    ui.label("Hit");
    let current = hit_type_label(*hit_type);
    egui::ComboBox::from_id_salt(salt)
        .selected_text(current)
        .show_ui(ui, |ui| {
            for template in HIT_TYPE_TEMPLATES {
                let label = hit_type_label(template);
                if ui.selectable_label(current == label, label).clicked() && current != label {
                    *hit_type = template;
                }
            }
        });
}

/// The per-variant [`HitType`] payload drags, editing the shape newtypes in place
/// through their constructors. The cell counts are the `u8` type's own range; the cone
/// half-angle (degrees) carries no documented bound.
fn hit_type_payload(ui: &mut egui::Ui, hit_type: &mut HitType) {
    match hit_type {
        HitType::Single => {}
        HitType::Blast { radius } => {
            ui.label("radius");
            let mut value = **radius;
            if ui.add(egui::DragValue::new(&mut value)).changed() {
                *radius = BlastRadius::new(value);
            }
        }
        HitType::Cone { range, angle } => {
            ui.label("range");
            let mut depth = **range;
            if ui.add(egui::DragValue::new(&mut depth)).changed() {
                *range = AoeRange::new(depth);
            }
            ui.label("angle°");
            let mut degrees = **angle;
            if ui
                .add(egui::DragValue::new(&mut degrees).speed(0.5))
                .changed()
            {
                *angle = ConeHalfAngle::new(degrees);
            }
        }
        HitType::Line { range } => {
            ui.label("range");
            let mut depth = **range;
            if ui.add(egui::DragValue::new(&mut depth)).changed() {
                *range = AoeRange::new(depth);
            }
        }
    }
}
