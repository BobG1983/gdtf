//! The MELEE form's FIGHT-MODE list editor (GTW-671 C2) — the melee SIBLING of the
//! shared [`fire_mode_edit`](crate::egui_shell::fire_mode_edit) row (the same look:
//! kind combo + labelled per-mode drags in one row), over the melee-only
//! [`FightModeSpec`] shape.
//!
//! Deliberately NOT the shared fire-mode widget itself: a [`FightModeSpec`] is a
//! DIFFERENT closed type (`Swing`/`Thrust` kind, a FLAT [`TuCost`] — not the ranged TU
//! pool fraction — a [`Strikes`] count, and NO `cone_mult`/hit-type: a melee strike has
//! no dispersion cone), so reusing the ranged row would author fields the melee spec
//! does not have. Single-consumer, so it lives with its consumer (module-layout
//! rule 6).
//!
//! The [`FightMode`] sim newtype is construct-only (a private inner with slice access),
//! so the list edits by PROJECTING the authored list out, mutating it, and folding it
//! back through the SAME constructor the loader's deserialize uses. Drag clamps are the
//! payload TYPES' own ranges (the GTW-479 no-invented-clamps ruling — the specs
//! document no tighter numeric bounds).

use bevy_egui::egui;
use gdtf_battle_sim::weapon::{
    FightMode, FightModeKind, FightModeSpec, MeleeWeaponSpec, Strikes, TuCost,
};

use crate::melee_weapon_form::structural_swing_mode;

/// The two closed [`FightModeKind`]s the kind combo offers, in the sim's declaration
/// order (GTW-505 — the swing/thrust vocabulary).
const FIGHT_MODE_KINDS: [FightModeKind; 2] = [FightModeKind::Swing, FightModeKind::Thrust];

/// The FIGHT-MODE list — one row per authored [`FightModeSpec`] (the kind combo plus
/// the `tu_cost` / `strikes` drags — the melee sibling of the shared fire-mode row)
/// plus Remove, and the Add-mode button seeding the structural single-swing template.
/// Remove is DISABLED at one mode: the [`FightMode`] invariant documents "a
/// well-authored melee weapon lists at least one mode", so a mode-less weapon is
/// unauthorable by construction (the GTW-670 fire-mode floor precedent).
pub(super) fn fight_modes_list(ui: &mut egui::Ui, spec: &mut MeleeWeaponSpec) {
    let mut modes: Vec<FightModeSpec> = spec.fight_mode.to_vec();
    let single_mode = modes.len() == 1;
    let mut remove: Option<usize> = None;
    for (index, mode) in modes.iter_mut().enumerate() {
        ui.group(|ui| {
            fight_mode_row(ui, index, mode);
            if ui
                .add_enabled(!single_mode, egui::Button::new("Remove mode"))
                .clicked()
            {
                remove = Some(index);
            }
        });
    }
    if let Some(index) = remove {
        modes.remove(index);
    }
    if ui.button("Add fight mode").clicked() {
        modes.push(structural_swing_mode());
    }
    // Fold the edited list back through the loader's own constructor.
    spec.fight_mode = FightMode::new(modes);
}

/// Draw one fight-mode row over `spec`, editing it IN PLACE: the [`FightModeKind`]
/// combo (the closed `Swing` / `Thrust` vocabulary — the authored RON variant names)
/// and drags for the two per-mode numbers, each folding through its sim newtype's
/// constructor. Salted by `index` (the row's position in the calling list) so several
/// rows coexist — the shared fire-mode row's exact convention.
fn fight_mode_row(ui: &mut egui::Ui, index: usize, spec: &mut FightModeSpec) {
    ui.horizontal(|ui| {
        ui.label("Kind");
        egui::ComboBox::from_id_salt(("fight_mode_kind", index))
            .selected_text(format!("{:?}", spec.kind))
            .show_ui(ui, |ui| {
                for option in FIGHT_MODE_KINDS {
                    ui.selectable_value(&mut spec.kind, option, format!("{option:?}"));
                }
            });
        ui.label("TU");
        let mut tu = *spec.tu_cost;
        if ui.add(egui::DragValue::new(&mut tu)).changed() {
            spec.tu_cost = TuCost::new(tu);
        }
        ui.label("strikes");
        let mut strikes = *spec.strikes;
        if ui.add(egui::DragValue::new(&mut strikes)).changed() {
            spec.strikes = Strikes::new(strikes);
        }
    });
}
