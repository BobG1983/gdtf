use std::path::Path;

use gdtf_battle_sim::{
    equipment::attachments::{
        AttachmentName, AttachmentSlot, FittedAttachments, SlotCapacity, WeaponSlots,
    },
    weapon::{
        DamageType, DotDamage, DotProfile, FireMode, FireModeSpec, Handedness, ModeConeMult,
        ModeKind, ModeShots, ModeTuPercent, TrajectoryStyle, WeaponName,
    },
};

use super::{
    draft::{WeaponDraft, dot_turns_from_raw, structural_single_mode},
    save::{draft_to_weapon_spec, weapon_file_name, weapon_save_path_in},
};

#[test]
fn new_weapon_seeds_the_structural_minimum() {
    let draft = WeaponDraft::new_weapon();
    let spec = draft.spec();
    assert_eq!(
        spec.fire_mode.as_slice(),
        &[structural_single_mode()],
        "a fresh weapon seeds exactly the structural single-shot mode",
    );
    assert_eq!(spec.damage_type, DamageType::Kinetic);
    assert_eq!(spec.handedness, Handedness::OneHanded);
    assert_eq!(spec.trajectory, TrajectoryStyle::Straight);
    assert!(!*spec.stable, "the stable tag seeds off");
    assert!(!*spec.shove, "the shove tag seeds off");
    assert!(spec.slots.declarations().is_empty(), "no slots offered");
    assert!(spec.attachments.is_empty(), "no attachment keys");
    assert!(spec.dot.is_none(), "no DOT profile");
    assert!(spec.on_death.is_none(), "no on-death effect");
}

#[test]
fn load_is_verbatim_and_projection_trims_the_name() {
    let mut authored = WeaponDraft::new_weapon();
    authored.spec_mut().handedness = Handedness::TwoHanded;
    authored.spec_mut().attachments =
        FittedAttachments::new(vec![AttachmentName::new("suppressor".to_owned())]);
    let authored = authored.spec().clone();
    let key = WeaponName::new("scrap_rifle".to_owned());

    let mut draft = WeaponDraft::default();
    assert!(draft.autoload_pending(), "a default draft awaits the seed");
    draft.load_weapon(&key, &authored);
    assert!(!draft.autoload_pending(), "a load ends the one-shot seed");
    assert_eq!(draft.name(), "scrap_rifle");
    assert_eq!(draft.spec(), &authored, "load copies the spec verbatim");

    draft.set_name("  re keyed  ".to_owned());
    let (name, spec) = draft_to_weapon_spec(&draft);
    assert_eq!(name.as_str(), "re keyed", "the projection trims the buffer");
    assert_eq!(
        &spec,
        draft.spec(),
        "the projection copies the working spec"
    );
}

#[test]
fn fire_mode_list_edits_fold_back_through_the_loader_ctor() {
    let mut draft = WeaponDraft::new_weapon();
    let mut modes: Vec<FireModeSpec> = draft.spec().fire_mode.to_vec();
    modes.push(FireModeSpec::new(
        ModeKind::Burst,
        ModeConeMult::new(1.3),
        ModeTuPercent::new(0.5),
        ModeShots::new(3),
    ));
    draft.spec_mut().fire_mode = FireMode::new(modes);
    assert_eq!(draft.spec().fire_mode.len(), 2, "the added row landed");

    let mut modes: Vec<FireModeSpec> = draft.spec().fire_mode.to_vec();
    modes.remove(1);
    draft.spec_mut().fire_mode = FireMode::new(modes);
    assert_eq!(
        draft.spec().fire_mode.as_slice(),
        &[structural_single_mode()],
        "the removal folded back to the seed row",
    );
}

#[test]
fn slots_edits_fold_back_through_the_loader_ctor() {
    let mut draft = WeaponDraft::new_weapon();
    let mut declarations = draft.spec().slots.declarations().to_vec();
    declarations.push((AttachmentSlot::Rail, SlotCapacity::new(3)));
    draft.spec_mut().slots = WeaponSlots::new(declarations);
    assert_eq!(
        draft.spec().slots.capacity(AttachmentSlot::Rail),
        Some(SlotCapacity::new(3)),
        "the authored (Rail, 3) declaration answers its capacity",
    );
    assert_eq!(draft.spec().slots.declarations().len(), 1);
}

#[test]
fn dot_turns_fold_floors_zero_at_one() {
    assert_eq!(dot_turns_from_raw(0).get(), 1, "zero floors to one turn");
    assert_eq!(dot_turns_from_raw(1).get(), 1);
    assert_eq!(dot_turns_from_raw(7).get(), 7, "a positive count survives");

    let seed = DotProfile::default();
    assert_eq!(seed.turns.get(), 1, "the DOT seed is the minimal ONE turn");
    assert_eq!(*seed.damage, *DotDamage::new(0));
}

#[test]
fn save_path_derives_from_the_family_consts() {
    let name = WeaponName::new("Mag-Lock Ripper (mk II)".to_owned());
    assert_eq!(weapon_file_name(&name), "mag_lock_ripper_mk_ii.weapon.ron");
    assert_eq!(
        weapon_save_path_in(Path::new("/tmp/assets"), &name),
        Path::new("/tmp/assets/content/weapons/ranged/mag_lock_ripper_mk_ii.weapon.ron"),
    );
    assert_eq!(
        weapon_file_name(&WeaponName::new("!!!".to_owned())),
        "unnamed_weapon.weapon.ron",
        "a name that sanitizes to nothing falls back to the documented stem",
    );
}

#[test]
fn the_last_fire_mode_cannot_be_removed() {
    let mut draft = WeaponDraft::new_weapon();
    assert_eq!(draft.fire_modes().len(), 1, "a fresh draft holds one mode");

    assert!(
        !draft.remove_fire_mode(0),
        "the only fire mode reports nothing removed",
    );
    assert_eq!(
        draft.fire_modes().len(),
        1,
        "the refused removal leaves the list untouched",
    );

    draft.add_fire_mode();
    assert!(
        draft.remove_fire_mode(0),
        "with two modes authored, one may go",
    );
    assert_eq!(
        draft.fire_modes().len(),
        1,
        "the removal stops at the structural minimum",
    );
}

#[test]
fn set_fire_mode_writes_only_in_bounds() {
    let mut draft = WeaponDraft::new_weapon();
    let burst = FireModeSpec::new(
        ModeKind::Burst,
        ModeConeMult::new(1.0),
        ModeTuPercent::new(0.0),
        ModeShots::new(3),
    );

    assert!(
        !draft.set_fire_mode(1, burst),
        "index 1 is past the end of a one-mode list",
    );
    assert_eq!(
        draft.fire_modes()[0].kind,
        ModeKind::Single,
        "the refused write left the seeded mode alone",
    );

    assert!(draft.set_fire_mode(0, burst), "index 0 is in bounds");
    assert_eq!(
        draft.fire_modes()[0].kind,
        ModeKind::Burst,
        "the in-bounds write folded back through the loader ctor",
    );
}
