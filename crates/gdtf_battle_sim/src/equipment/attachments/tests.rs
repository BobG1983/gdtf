use bevy::prelude::{App, Entity, Update, World};

use super::{
    AttachToWeaponExt, AttachmentName, AttachmentRegistry, AttachmentSlot, AttachmentSpec,
    FittedAttachments, SlotCapacity, WeaponSlots, apply_pending_attachments,
};
use crate::{
    effects::attachments::{AimDelta, AttachmentEffect},
    magazine::{Magazine, ReloadTu},
    weapon::{Accuracy, DamageType, MagazineSize, PendingAttachments, Silenced, WeaponName},
};

#[test]
fn attachment_spec_parses_with_effect_list() {
    let ron = "(display_name: \"Whisper Bore\", slot: Muzzle, effects: [Silence, Aim(0.2)])";
    let Ok(spec) = ron::de::from_str::<AttachmentSpec>(ron) else {
        unreachable!("an AttachmentSpec with a display_name + slot + effects list must parse");
    };
    assert_eq!(
        spec.display_name,
        WeaponName::new("Whisper Bore".to_owned()),
        "the display_name round-trips"
    );
    assert_eq!(
        spec.slot,
        AttachmentSlot::Muzzle,
        "the slot the item occupies round-trips"
    );
    assert_eq!(
        spec.effects.len(),
        2,
        "both authored effects parse into the list"
    );
    assert!(
        matches!(spec.effects.first(), Some(AttachmentEffect::Silence)),
        "the first effect is the no-payload Silence"
    );
}

/// (the `#[serde(default)]` identity) — a weapon fitting it applies nothing. The `slot`
#[test]
fn attachment_spec_defaults_to_empty_effects() {
    let Ok(spec) = ron::de::from_str::<AttachmentSpec>("(display_name: \"Bare Rail\", slot: Rail)")
    else {
        unreachable!("an AttachmentSpec omitting `effects:` must parse (serde default)");
    };
    assert!(
        spec.effects.is_empty(),
        "an omitted effects list defaults to empty (identity)"
    );
    assert!(
        ron::de::from_str::<AttachmentSpec>("(display_name: \"No Slot\")").is_err(),
        "an item omitting the REQUIRED `slot:` field fails to parse"
    );
}

#[test]
fn registry_keys_and_looks_up_by_name() {
    let Ok(spec) = ron::de::from_str::<AttachmentSpec>(
        "(display_name: \"Scoped Sight\", slot: Sight, effects: [Aim(0.4)])",
    ) else {
        unreachable!("the fixture spec must parse");
    };
    let key = AttachmentName::new("scoped_sight".to_owned());
    let registry = AttachmentRegistry::new([(key.clone(), spec)]);

    assert_eq!(
        registry.len(),
        1,
        "the registry holds the one inserted item"
    );
    assert!(
        registry.spec(&key).is_some(),
        "an inserted key resolves to its spec"
    );
    assert!(
        registry
            .spec(&AttachmentName::new("missing".to_owned()))
            .is_none(),
        "a missing key fails closed with None"
    );
}

fn spawn_weapon(world: &mut World) -> Entity {
    world
        .spawn((
            Accuracy::new(1.0),
            DamageType::Kinetic,
            Magazine::loaded(MagazineSize::new(20), ReloadTu::new(20)),
        ))
        .id()
}

#[test]
fn empty_effect_list_is_the_identity() {
    let mut world = World::new();
    let weapon = spawn_weapon(&mut world);
    let before_acc = world.entity(weapon).get::<Accuracy>().copied();
    let before_mag = world.entity(weapon).get::<Magazine>().copied();

    let effects: Vec<AttachmentEffect> = Vec::new();
    for effect in &effects {
        world.commands().attach_to_weapon(weapon, effect.clone());
    }
    world.flush();

    assert_eq!(
        world.entity(weapon).get::<Accuracy>().copied(),
        before_acc,
        "no effects leaves Accuracy unchanged"
    );
    assert_eq!(
        world.entity(weapon).get::<Magazine>().copied(),
        before_mag,
        "no effects leaves the Magazine unchanged"
    );
    assert!(
        world.entity(weapon).get::<Silenced>().is_none(),
        "no effects adds no sibling tags"
    );
}

#[test]
fn absent_target_component_is_a_noop() {
    let mut world = World::new();
    let weapon = world.spawn_empty().id();
    world
        .commands()
        .attach_to_weapon(weapon, AttachmentEffect::Aim(AimDelta::new(0.4)));
    world.flush();
    assert!(
        world.entity(weapon).get::<Accuracy>().is_none(),
        "Aim on a weapon with no Accuracy is a fail-safe no-op (no panic, no insert)"
    );
}

fn app_with_pending(effects: Vec<AttachmentEffect>) -> (App, Entity) {
    let mut app = App::new();
    app.add_systems(Update, apply_pending_attachments);
    let weapon = app
        .world_mut()
        .spawn((
            Accuracy::new(1.0),
            Magazine::loaded(MagazineSize::new(20), ReloadTu::new(20)),
            PendingAttachments::new(effects),
        ))
        .id();
    (app, weapon)
}

#[test]
fn applies_pending_effects_then_removes_the_marker() {
    let (mut app, weapon) = app_with_pending(vec![AttachmentEffect::Aim(AimDelta::new(0.4))]);
    app.update();

    let Some(accuracy) = app.world().entity(weapon).get::<Accuracy>() else {
        unreachable!("the weapon must still carry Accuracy");
    };
    assert!(
        **accuracy > 1.0,
        "the pending Aim effect must raise Accuracy above the 1.0 baseline (got {})",
        **accuracy,
    );
    assert!(
        app.world()
            .entity(weapon)
            .get::<PendingAttachments>()
            .is_none(),
        "the PendingAttachments marker must be removed after application (one-shot)",
    );
}

#[test]
fn applies_every_pending_effect() {
    let (mut app, weapon) = app_with_pending(vec![
        AttachmentEffect::Silence,
        AttachmentEffect::ExtraAmmo(MagazineSize::new(10)),
    ]);
    app.update();

    assert!(
        app.world().entity(weapon).get::<Silenced>().is_some(),
        "the pending Silence effect must insert the Silenced tag",
    );
    let Some(magazine) = app.world().entity(weapon).get::<Magazine>() else {
        unreachable!("the weapon must still carry a Magazine");
    };
    assert!(
        magazine.size().get() > 20,
        "the pending ExtraAmmo effect must grow the magazine above the 20 baseline (got {})",
        magazine.size().get(),
    );
}

#[test]
fn empty_pending_list_is_the_identity() {
    let (mut app, weapon) = app_with_pending(Vec::new());
    app.update();

    let Some(accuracy) = app.world().entity(weapon).get::<Accuracy>() else {
        unreachable!("the weapon must still carry Accuracy");
    };
    assert!(
        (**accuracy - 1.0).abs() < f32::EPSILON,
        "an empty pending list leaves Accuracy at the 1.0 baseline (got {})",
        **accuracy,
    );
    assert!(
        app.world()
            .entity(weapon)
            .get::<PendingAttachments>()
            .is_none(),
        "the (empty) PendingAttachments marker is still removed (one-shot)",
    );
}

#[test]
fn slot_declarations_add_and_remove_by_index() {
    let mut slots = WeaponSlots::default();

    slots.add_slot();
    assert_eq!(
        slots.declarations().len(),
        1,
        "add_slot appends exactly one declaration",
    );
    assert_eq!(
        slots.capacity(AttachmentSlot::Muzzle),
        Some(SlotCapacity::new(1)),
        "the appended declaration is the default Muzzle/1",
    );

    assert!(
        slots.set_slot(0, AttachmentSlot::Rail, SlotCapacity::new(3)),
        "set_slot writes in bounds",
    );
    assert_eq!(
        slots.capacity(AttachmentSlot::Muzzle),
        None,
        "the rewritten declaration no longer answers for Muzzle",
    );
    assert_eq!(
        slots.capacity(AttachmentSlot::Rail),
        Some(SlotCapacity::new(3)),
        "the rewritten declaration answers for Rail at the written capacity",
    );

    assert!(
        !slots.remove_slot(1),
        "remove_slot refuses an out-of-range index",
    );
    assert!(slots.remove_slot(0), "remove_slot removes the only row");
    assert!(
        slots.declarations().is_empty(),
        "a slot list with no minimum empties out",
    );
}

#[test]
fn fitted_attachments_add_and_remove_by_index() {
    let mut fitted = FittedAttachments::default();
    let first = AttachmentName::new("suppressor".to_owned());
    let second = AttachmentName::new("bionic_sight".to_owned());

    fitted.add(first);
    assert_eq!(fitted.len(), 1, "add appends exactly one key");

    assert!(fitted.set(0, second.clone()), "set writes in bounds");
    assert_eq!(
        fitted.first(),
        Some(&second),
        "the written key replaced the appended one",
    );
    assert!(!fitted.set(1, second), "set refuses an out-of-range index");

    assert!(fitted.remove(0), "remove takes the only key out");
    assert!(fitted.is_empty(), "the list has no minimum");
    assert!(
        !fitted.remove(0),
        "removing from an empty list reports nothing removed",
    );
}
