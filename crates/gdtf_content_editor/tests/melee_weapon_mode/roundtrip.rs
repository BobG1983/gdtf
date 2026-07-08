//! GTW-671 C4: the MELEE mode's REAL round-trips — author a MAXIMAL spec (multi fight
//! modes, a multi-slot declaration list, attachment keys, and every scalar non-default)
//! AND a MINIMAL spec (the serde-default identities exercised) in the form model, save
//! both through the REAL root-parameterized write (`write_melee_weapon_in`) into a
//! `TempDir` assets root (the GTW-555 pattern), then boot the REAL editor app rooted at
//! that directory and assert the actual `MeleeWeaponsFamily` folder walk loads both
//! saved weapons back structurally identical.
//!
//! Also pins the GTW-671 lifecycle riders: the editor reaches `Editing` with the
//! `MeleeWeaponRegistry` gate resource present and the state-scoped `MeleeWeaponDraft`
//! seeded (salvage / fallback behavior itself is the seam's parameterized family
//! contract — `register_content_family::<MeleeWeaponsFamily>` inherits it, no
//! per-family re-pin here). RE-VALIDATION on a melee save is FREE and already pinned:
//! the `MeleeWeaponRegistry` is in the editor's re-arm watch set (`validate/rearm.rs`
//! `WatchedRegistries::melee_weapons`, GTW-651) and the weapon→attachment edge chains
//! the melee registry (`gdtf_content_families/validate/attachments.rs`, GTW-669 — its
//! suite lives in `tests/authoring_validation/attachments.rs`) — no new machinery here.

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

/// The MAXIMAL edited melee weapon, authored through the REAL form-model mutators:
/// every one of the spec's 11 authored fields carries a NON-default value — two fight
/// modes (one `Swing`, one `Thrust`), a `(Counterweight, 1)` + `(Pommel, 2)` slot list,
/// two attachment keys, a reach above the default, and the shove tag ON — so a single
/// dropped or drifted field cannot round-trip (the gate's field-for-field spot-check
/// target).
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

/// The MINIMAL edited melee weapon: only the REQUIRED fields carry non-seed values;
/// every opt-in field stays at its serde-default identity (reach 1, shove off, no
/// slots, no attachments) — so the round-trip exercises the defaults through the real
/// serialize → parse chain.
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

/// GTW-671 C4 — create → save BOTH specs (the REAL write into a `TempDir` assets root)
/// → load through the REAL `MeleeWeaponsFamily` folder walk → the registry holds the
/// SAME specs (structural equality across all 11 authored fields: the maximal record
/// and the defaults-exercising minimal record), with the `Editing` gate + the scoped
/// `MeleeWeaponDraft` seed along for the ride.
#[test]
fn saved_melee_weapons_round_trip_through_the_real_melee_loader() {
    let dir = tempfile::tempdir();
    assert!(dir.is_ok(), "creating the TempDir assets root must succeed");
    let Ok(dir) = dir else { return };

    // SAVE both drafts through the real root-parameterized write.
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

    // RELOAD through the real editor Load pass rooted at the TempDir.
    let mut app = editor_app_with_asset_root(dir.path());
    advance_to_editing(&mut app);

    let world = app.world();
    // The state-scoped MELEE draft seeded on entering Editing (bevy-traps #1 via the
    // GTW-575 seam) — the A2 state-scoped pin's in-state half; the full
    // absent→seeded→removed lifecycle rides the `state_scoped_resources` suite.
    assert!(
        world.get_resource::<MeleeWeaponDraft>().is_some(),
        "the MeleeWeaponDraft must be seeded OnEnter(Editing)",
    );

    // The REAL folder walk keyed both saved files by their stems and loaded the SAME
    // specs.
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
