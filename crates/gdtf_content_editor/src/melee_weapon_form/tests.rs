//! GTW-671 C4: form-model headless tests over the MELEE-WEAPON draft's pure mutators,
//! the construct-only list-edit fold-backs, and the path derivation (the GTW-670 weapon
//! form-test parity).

use std::path::Path;

use gdtf_battle_sim::{
    equipment::attachments::{AttachmentName, AttachmentSlot, SlotCapacity, WeaponSlots},
    weapon::{
        DamageType, FightMode, FightModeKind, FightModeSpec, Handedness, Reach, Strikes, TuCost,
        WeaponName,
    },
};

use super::{
    draft::{MeleeWeaponDraft, structural_swing_mode},
    save::{draft_to_melee_weapon_spec, melee_weapon_file_name, melee_weapon_save_path_in},
};

/// A fresh draft seeds the structural minimum: ONE `Swing` fight mode (the documented
/// [`FightMode`] "at least one mode" invariant — the [`FightMode::primary`] fallback
/// shape), the serde-default identities for every opt-in field (reach 1 / shove off /
/// no slots / no attachment keys), and the documented vocabulary defaults.
#[test]
fn new_melee_weapon_seeds_the_structural_minimum() {
    let draft = MeleeWeaponDraft::new_melee_weapon();
    let spec = draft.spec();
    assert_eq!(
        spec.fight_mode.as_slice(),
        &[structural_swing_mode()],
        "a fresh melee weapon seeds exactly the structural single-swing mode",
    );
    assert_eq!(spec.damage_type, DamageType::Kinetic);
    assert_eq!(spec.handedness, Handedness::OneHanded);
    assert_eq!(
        spec.reach,
        Reach::DEFAULT,
        "the adjacent-cell default reach"
    );
    assert!(!*spec.shove, "the shove tag seeds off");
    assert!(spec.slots.declarations().is_empty(), "no slots offered");
    assert!(spec.attachments.is_empty(), "no attachment keys");
}

/// The load path copies an authored spec VERBATIM and ends the one-shot autoload; the
/// projection trims the name buffer into the registry-key [`WeaponName`] and copies the
/// working spec.
#[test]
fn load_is_verbatim_and_projection_trims_the_name() {
    let mut authored = MeleeWeaponDraft::new_melee_weapon();
    authored.spec_mut().handedness = Handedness::TwoHanded;
    authored.spec_mut().reach = Reach::new(2);
    authored.spec_mut().attachments = vec![AttachmentName::new("butchers_weight".to_owned())];
    let authored = authored.spec().clone();
    let key = WeaponName::new("pit_cleaver".to_owned());

    let mut draft = MeleeWeaponDraft::default();
    assert!(draft.autoload_pending(), "a default draft awaits the seed");
    draft.load_melee_weapon(&key, &authored);
    assert!(!draft.autoload_pending(), "a load ends the one-shot seed");
    assert_eq!(draft.name(), "pit_cleaver");
    assert_eq!(draft.spec(), &authored, "load copies the spec verbatim");

    draft.set_name("  re keyed  ".to_owned());
    let (name, spec) = draft_to_melee_weapon_spec(&draft);
    assert_eq!(name.as_str(), "re keyed", "the projection trims the buffer");
    assert_eq!(
        &spec,
        draft.spec(),
        "the projection copies the working spec"
    );
}

/// The FIGHT-MODE list edits by projecting the authored list out, mutating it, and
/// folding it back through [`FightMode::new`] — the exact model path the form's
/// add/remove rows run (the sim newtype is construct-only, GTW-671 C2).
#[test]
fn fight_mode_list_edits_fold_back_through_the_loader_ctor() {
    let mut draft = MeleeWeaponDraft::new_melee_weapon();
    let mut modes: Vec<FightModeSpec> = draft.spec().fight_mode.to_vec();
    modes.push(FightModeSpec::new(
        FightModeKind::Thrust,
        TuCost::new(15),
        Strikes::new(2),
    ));
    draft.spec_mut().fight_mode = FightMode::new(modes);
    assert_eq!(draft.spec().fight_mode.len(), 2, "the added row landed");

    // Remove the added row again — the one-mode floor is form policy, so the model
    // path itself stays symmetric.
    let mut modes: Vec<FightModeSpec> = draft.spec().fight_mode.to_vec();
    modes.remove(1);
    draft.spec_mut().fight_mode = FightMode::new(modes);
    assert_eq!(
        draft.spec().fight_mode.as_slice(),
        &[structural_swing_mode()],
        "the removal folded back to the seed row",
    );
}

/// The SLOTS declaration edits by projecting [`WeaponSlots::declarations`] out and
/// folding back through [`WeaponSlots::new`] — the form's add/remove rows' model path,
/// including the melee-typical `Counterweight` / `Pommel` pair the shipped chainsword
/// authors (the closed palette needs no melee-only gate — class gating EMERGES from the
/// declarations, GTW-554).
#[test]
fn slots_edits_fold_back_through_the_loader_ctor() {
    let mut draft = MeleeWeaponDraft::new_melee_weapon();
    let mut declarations = draft.spec().slots.declarations().to_vec();
    declarations.push((AttachmentSlot::Counterweight, SlotCapacity::new(1)));
    declarations.push((AttachmentSlot::Pommel, SlotCapacity::new(1)));
    draft.spec_mut().slots = WeaponSlots::new(declarations);
    assert_eq!(
        draft.spec().slots.capacity(AttachmentSlot::Counterweight),
        Some(SlotCapacity::new(1)),
        "the authored (Counterweight, 1) declaration answers its capacity",
    );
    assert_eq!(
        draft.spec().slots.capacity(AttachmentSlot::Pommel),
        Some(SlotCapacity::new(1)),
    );
    assert_eq!(draft.spec().slots.declarations().len(), 2);
}

/// The path derivation runs through the family consts + the shared sanitize seam: the
/// literal `content/weapons/melee/<stem>.melee_weapon.ron` shape the GTW-505 loader
/// dispatches on (pinning the LITERALS is the drift alarm — GTW-621), with the
/// `unnamed_melee_weapon` fallback for a name that sanitizes to nothing.
#[test]
fn save_path_derives_from_the_family_consts() {
    let name = WeaponName::new("Pit-Fighter's Cleaver (mk II)".to_owned());
    assert_eq!(
        melee_weapon_file_name(&name),
        "pit_fighters_cleaver_mk_ii.melee_weapon.ron"
    );
    assert_eq!(
        melee_weapon_save_path_in(Path::new("/tmp/assets"), &name),
        Path::new("/tmp/assets/content/weapons/melee/pit_fighters_cleaver_mk_ii.melee_weapon.ron"),
    );
    assert_eq!(
        melee_weapon_file_name(&WeaponName::new("!!!".to_owned())),
        "unnamed_melee_weapon.melee_weapon.ron",
        "a name that sanitizes to nothing falls back to the documented stem",
    );
}
