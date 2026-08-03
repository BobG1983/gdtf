use gdtf_battle_sim::{
    effects::attachments::{AimDelta, AttachmentEffect},
    equipment::attachments::{AttachmentName, AttachmentSlot, AttachmentSpec, SlotCapacity},
    weapon::WeaponSpec,
};


#[test]
fn attachment_spec_parses_effects_from_ron() {
    let ron = r#"(
        display_name: "Bionic Sight",
        slot: Sight,
        effects: [ Aim(0.4), Silence, Penetration(6) ],
    )"#;
    let Ok(spec) = ron::from_str::<AttachmentSpec>(ron) else {
        unreachable!("an AttachmentSpec must parse its effects list from RON");
    };
    assert_eq!(
        spec.slot,
        AttachmentSlot::Sight,
        "the GTW-554 slot the item occupies parses from RON",
    );
    assert_eq!(
        spec.effects.len(),
        3,
        "the authored three-effect list round-trips"
    );
    assert_eq!(
        spec.effects[0],
        AttachmentEffect::Aim(AimDelta::new(0.4)),
        "the first effect parses as Aim with its per-item AimDelta payload",
    );
    assert_eq!(
        spec.effects[1],
        AttachmentEffect::Silence,
        "the second effect parses as the no-payload Silence variant",
    );
}

#[test]
fn weapon_attachments_and_omitted_field_parse_from_ron() {
    let with = r#"(
        base_spread: 0.05, accuracy: 5.0, kickback: 0.0, fatal_bias: 3.0,
        damage: 12, punch: 10, shred: 3, damage_type: Kinetic,
        magazine: (size: 20, reload_tu: 20),
        fire_mode: [(kind: Single, cone_mult: 1.0, tu_percent: 0.2, shots: 1)],
        stable: false, handedness: OneHanded,
        slots: [(Sight, 1), (Muzzle, 1)],
        attachments: ["bionic_sight", "suppressor"],
    )"#;
    let Ok(spec) = ron::from_str::<WeaponSpec>(with) else {
        unreachable!("a weapon's attachments key list must parse from RON");
    };
    assert_eq!(
        spec.attachments.len(),
        2,
        "the authored two-key list round-trips"
    );
    assert_eq!(
        spec.slots.capacity(AttachmentSlot::Sight),
        Some(SlotCapacity::new(1)),
        "the GTW-554 slots pair-list parses from the weapon RON",
    );
    assert_eq!(
        spec.attachments[0],
        AttachmentName::new("bionic_sight".to_owned()),
        "the first key parses as a bare RON string",
    );

    // No `attachments:` field — the `#[serde(default)]` opt-in yields an EMPTY list.
    let without = r"(
        base_spread: 0.05, accuracy: 5.0, kickback: 0.0, fatal_bias: 3.0,
        damage: 12, punch: 10, shred: 3, damage_type: Kinetic,
        magazine: (size: 20, reload_tu: 20),
        fire_mode: [(kind: Single, cone_mult: 1.0, tu_percent: 0.2, shots: 1)],
        stable: false, handedness: OneHanded,
    )";
    let Ok(spec) = ron::from_str::<WeaponSpec>(without) else {
        unreachable!("a weapon with no attachments field parses");
    };
    assert!(
        spec.attachments.is_empty(),
        "an omitted attachments field defaults to an empty list (the opt-in default)",
    );
}
