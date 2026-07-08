//! The REUSABLE **shared damage-group editor** (GTW-671 C2) — the egui controls that
//! edit the six fields the ranged [`WeaponSpec`](gdtf_battle_sim::weapon::WeaponSpec)
//! and the melee [`MeleeWeaponSpec`](gdtf_battle_sim::weapon::MeleeWeaponSpec) SHARE
//! verbatim: `damage` / `punch` / `shred` / `damage_type` / `fatal_bias` /
//! `handedness` (the sim's "shared ranged damage model" — both specs resolve through
//! the SAME per-hit formula).
//!
//! A SHARED `egui_shell` sibling (the [`fire_mode_edit`](super::fire_mode_edit)
//! precedent) on purpose: the WEAPON form (GTW-670) and the MELEE-WEAPON form (GTW-671)
//! both consume THIS widget, so the shared six fields have ONE authoring surface —
//! extracted here out of `weapon_form_ui/stats.rs`'s stats + handling groups rather
//! than forked with silently divergent clamps. The generic scalar drags + the
//! [`DamageType`] combo also live here (2+ consuming modules — module-layout rule 6):
//! the ranged form's remaining ballistics scalars and its DOT sub-form reuse them.
//!
//! NO drag clamp is invented (the GTW-479 ruling): every magnitude is documented
//! tuning-open, so the drags span their payload TYPES' own ranges; the signed damage
//! triple spans the honest `i32` the per-hit formula subtracts with.

use bevy_egui::egui;
use gdtf_battle_sim::weapon::{
    DamageType, FatalBias, Handedness, WeaponDamage, WeaponPunch, WeaponShred,
};

/// The two closed [`Handedness`] variants the combo offers, in the sim's declaration
/// order (GTW-443 — the hand-count vocabulary).
const HANDEDNESS_OPTIONS: [Handedness; 2] = [Handedness::OneHanded, Handedness::TwoHanded];

/// The six SHARED damage-group borrows the widget edits — projected out of whichever
/// spec record the calling form holds (the ranged `WeaponSpec` or the melee
/// `MeleeWeaponSpec`; both own the same six newtypes, so the projection is six disjoint
/// field borrows). A borrowed-fields bundle (the `ModePanelsCtx` shape) rather than six
/// parameters, so the widget's signature stays legible.
pub(in crate::egui_shell) struct DamageGroupFields<'a> {
    /// The base damage a strike/hit deals before armor (`damage`).
    pub(in crate::egui_shell) damage:      &'a mut WeaponDamage,
    /// The armor protection a hit ignores — penetration (`punch`).
    pub(in crate::egui_shell) punch:       &'a mut WeaponPunch,
    /// The extra integrity damage a hit deals to armor durability (`shred`).
    pub(in crate::egui_shell) shred:       &'a mut WeaponShred,
    /// The damage type the weapon emits — its matchup-wheel node (`damage_type`).
    pub(in crate::egui_shell) damage_type: &'a mut DamageType,
    /// The severity-score addend, consumed by E3 (`fatal_bias`).
    pub(in crate::egui_shell) fatal_bias:  &'a mut FatalBias,
    /// The weapon's [`Handedness`] (`handedness`).
    pub(in crate::egui_shell) handedness:  &'a mut Handedness,
}

/// Draw the SHARED damage group — the six fields in the melee spec's documented order
/// (`damage` / `punch` / `shred` / `damage_type` / `fatal_bias` / `handedness`), each
/// committing through its sim newtype's constructor (GTW-671 C2). `salt` is the calling
/// form's id prefix (`"weapon"` / `"melee_weapon"`) so both forms' combos coexist.
pub(in crate::egui_shell) fn damage_group(
    ui: &mut egui::Ui,
    salt: &'static str,
    fields: DamageGroupFields<'_>,
) {
    drag_i32(ui, "damage", **fields.damage, |v| {
        *fields.damage = WeaponDamage::new(v);
    });
    drag_i32(ui, "punch", **fields.punch, |v| {
        *fields.punch = WeaponPunch::new(v);
    });
    drag_i32(ui, "shred", **fields.shred, |v| {
        *fields.shred = WeaponShred::new(v);
    });
    damage_type_combo(ui, (salt, "damage_type"), fields.damage_type);
    drag_f32(ui, "fatal_bias", 0.05, **fields.fatal_bias, |v| {
        *fields.fatal_bias = FatalBias::new(v);
    });
    ui.horizontal(|ui| {
        ui.label("handedness");
        egui::ComboBox::from_id_salt((salt, "handedness"))
            .selected_text(format!("{:?}", fields.handedness))
            .show_ui(ui, |ui| {
                for option in HANDEDNESS_OPTIONS {
                    ui.selectable_value(fields.handedness, option, format!("{option:?}"));
                }
            });
    });
}

/// A labelled [`DamageType`] combo over the seven wheel nodes in the sim's canonical
/// [`DamageType::ALL`] order (the attachment form's convention: debug-formatted labels
/// ARE the authored RON identifiers). Shared by both forms' damage groups and the
/// ranged DOT sub-form (distinct salts).
pub(in crate::egui_shell) fn damage_type_combo(
    ui: &mut egui::Ui,
    salt: impl std::hash::Hash + std::fmt::Debug,
    ty: &mut DamageType,
) {
    ui.horizontal(|ui| {
        ui.label("damage_type");
        egui::ComboBox::from_id_salt(salt)
            .selected_text(format!("{ty:?}"))
            .show_ui(ui, |ui| {
                for option in DamageType::ALL {
                    ui.selectable_value(ty, option, format!("{option:?}"));
                }
            });
    });
}

/// A labelled `f32` stat drag on its own row, committing through `commit` only on a
/// change (the attachment payload-drag shape).
pub(in crate::egui_shell) fn drag_f32(
    ui: &mut egui::Ui,
    label: &str,
    speed: f64,
    value: f32,
    commit: impl FnOnce(f32),
) {
    ui.horizontal(|ui| {
        ui.label(label);
        let mut edited = value;
        if ui
            .add(egui::DragValue::new(&mut edited).speed(speed))
            .changed()
        {
            commit(edited);
        }
    });
}

/// A labelled `i32` stat drag — the signed-magnitude twin of [`drag_f32`] (damage /
/// punch / shred share the sim's honest-signed `i32`).
pub(in crate::egui_shell) fn drag_i32(
    ui: &mut egui::Ui,
    label: &str,
    value: i32,
    commit: impl FnOnce(i32),
) {
    ui.horizontal(|ui| {
        ui.label(label);
        let mut edited = value;
        if ui.add(egui::DragValue::new(&mut edited)).changed() {
            commit(edited);
        }
    });
}
