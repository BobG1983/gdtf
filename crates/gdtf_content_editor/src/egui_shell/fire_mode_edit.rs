//! Shared fire-mode editor widget for weapon and attachment forms.
use bevy_egui::egui;
use gdtf_battle_sim::weapon::{
    AoeRange, BlastRadius, ConeHalfAngle, FireModeSpec, HitType, ModeConeMult, ModeKind, ModeShots,
    ModeTuPercent,
};

const MODE_KINDS: [ModeKind; 3] = [ModeKind::Single, ModeKind::Burst, ModeKind::Full];

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

const fn hit_type_label(hit_type: HitType) -> &'static str {
    match hit_type {
        HitType::Single => "Single",
        HitType::Blast { .. } => "Blast",
        HitType::Cone { .. } => "Cone",
        HitType::Line { .. } => "Line",
    }
}

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
