//! GTW-671 C4: the MELEE mode's REAL round-trips — author a MAXIMAL spec (multi fight
use gdtf_battle_sim::{
    equipment::attachments::{AttachmentName, AttachmentSlot, SlotCapacity, WeaponSlots},
    weapon::{
        DamageType, FatalBias, FightMode, FightModeKind, FightModeSpec, Handedness,
        MeleeWeaponRegistry, Reach, Shove, Strikes, TuCost, WeaponDamage, WeaponName, WeaponPunch,
        WeaponShred,
    },
};
use gdtf_content_editor::{MeleeWeaponDraft, draft_to_melee_weapon_spec, write_melee_weapon_in};

use crate::harness::{advance_to_editing, editor_app_with_asset_root};

fn maximal_draft() -> MeleeWeaponDraft {
    let mut draft = MeleeWeaponDraft::new_melee_weapon();
    draft.set_name("tempdir_pit_cleaver".to_owned());
    let spec = draft.spec_mut();
    spec.damage = WeaponDamage::new(9);
    spec.punch = WeaponPunch::new(4);
    spec.shred = WeaponShred::new(7);
    spec.damage_type = DamageType::Rend;
    spec.fatal_bias = FatalBias::new(2.5);
    spec.handedness = Handedness::TwoHanded;
    spec.reach = Reach::new(3);
    spec.fight_mode = FightMode::new(vec![
        FightModeSpec::new(FightModeKind::Swing, TuCost::new(30), Strikes::new(1)),
        FightModeSpec::new(FightModeKind::Thrust, TuCost::new(18), Strikes::new(2)),
    ]);
    spec.shove = Shove::new(true);
    spec.slots = WeaponSlots::new(vec![
        (AttachmentSlot::Counterweight, SlotCapacity::new(1)),
        (AttachmentSlot::Pommel, SlotCapacity::new(2)),
    ]);
    spec.attachments = vec![
        AttachmentName::new("butchers_weight".to_owned()),
        AttachmentName::new("serrated_edge".to_owned()),
    ];
    draft
}

fn minimal_draft() -> MeleeWeaponDraft {
    let mut draft = MeleeWeaponDraft::new_melee_weapon();
    draft.set_name("tempdir_shiv".to_owned());
    let spec = draft.spec_mut();
    spec.damage = WeaponDamage::new(3);
    spec.punch = WeaponPunch::new(1);
    spec.fight_mode = FightMode::new(vec![FightModeSpec::new(
        FightModeKind::Thrust,
        TuCost::new(12),
        Strikes::new(1),
    )]);
    draft
}

#[test]
fn saved_melee_weapons_round_trip_through_the_real_melee_loader() {
    let dir = tempfile::tempdir();
    assert!(dir.is_ok(), "creating the TempDir assets root must succeed");
    let Ok(dir) = dir else { return };

    let (max_name, max_spec) = draft_to_melee_weapon_spec(&maximal_draft());
    let written = write_melee_weapon_in(dir.path(), &max_name, &max_spec);
    assert!(
        written.is_ok(),
        "the real maximal melee-weapon write must succeed: {:?}",
        written.as_ref().err(),
    );
    let (min_name, min_spec) = draft_to_melee_weapon_spec(&minimal_draft());
    let written = write_melee_weapon_in(dir.path(), &min_name, &min_spec);
    assert!(
        written.is_ok(),
        "the real minimal melee-weapon write must succeed: {:?}",
        written.as_ref().err(),
    );

    let mut app = editor_app_with_asset_root(dir.path());
    advance_to_editing(&mut app);

    let world = app.world();
    assert!(
        world.get_resource::<MeleeWeaponDraft>().is_some(),
        "the MeleeWeaponDraft must be seeded OnEnter(Editing)",
    );

    let registry = world.get_resource::<MeleeWeaponRegistry>();
    assert!(registry.is_some(), "the MeleeWeaponRegistry must resolve");
    let Some(registry) = registry else { return };
    assert_eq!(
        registry.spec(&WeaponName::new("tempdir_pit_cleaver".to_owned())),
        Some(&max_spec),
        "the reloaded MAXIMAL melee weapon must equal the saved spec field-for-field \
         (both fight modes incl. Thrust TU/strikes, the (Counterweight,1)+(Pommel,2) \
         slots, both attachment keys, reach 3, shove ON, and every damage scalar) — \
         the GTW-257 stem-key round-trip through the REAL loader",
    );
    assert_eq!(
        registry.spec(&WeaponName::new("tempdir_shiv".to_owned())),
        Some(&min_spec),
        "the reloaded MINIMAL melee weapon must equal the saved spec — the \
         serde-default identities (reach 1 / shove off / no slots / no attachments) \
         survive the serialize → parse chain",
    );
}
