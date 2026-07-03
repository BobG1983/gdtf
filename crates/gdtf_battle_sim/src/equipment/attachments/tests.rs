//! Mechanics tests for the attachment MECHANICS module (GTW-558): the loader-facing schema
//! ([`AttachmentSpec`](super::AttachmentSpec) parses; the
//! [`AttachmentRegistry`](super::AttachmentRegistry) keys + looks up by name), the
//! [`attach_to_weapon`](super::AttachToWeaponExt::attach_to_weapon) commands-extension apply
//! path (queue → flush; empty-list identity; absent-target no-op), and the post-spawn
//! [`apply_pending_attachments`](super::apply_pending_attachments) system (applies each
//! effect + removes the one-shot marker).
//!
//! Per-effect stat mapping is asserted in each effect file under
//! [`crate::effects::attachments`]; these prove the MECHANISM (schema round-trip, generic
//! trait invocation, the deferred command + spawn-applier), not any shipped magnitude.

use bevy::prelude::{App, Entity, Update, World};

use super::{
    AttachToWeaponExt, AttachmentName, AttachmentRegistry, AttachmentSlot, AttachmentSpec,
    apply_pending_attachments,
};
use crate::{
    effects::attachments::{AimDelta, AttachmentEffect},
    magazine::{Magazine, ReloadTu},
    weapon::{Accuracy, DamageType, MagazineSize, PendingAttachments, Silenced, WeaponName},
};

// --- schema round-trip (loader-facing) ---------------------------------------------------

/// An [`AttachmentSpec`] parses from a RON item carrying a `display_name` + a `slot` + an
/// `effects` list of mixed variants. Pin-discriminating: a schema mismatch fails to parse.
/// Asserts the list is populated (mechanism), not the magnitudes.
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
        "the GTW-554 slot the item occupies round-trips"
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

/// A cosmetic [`AttachmentSpec`] that authors NO `effects:` field parses to an EMPTY list
/// (the `#[serde(default)]` identity) — a weapon fitting it applies nothing. The `slot`
/// field stays REQUIRED (GTW-554): an item that omits it must FAIL to parse (every item
/// declares its mount point).
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
        "an item omitting the REQUIRED `slot:` field fails to parse (GTW-554)"
    );
}

/// The [`AttachmentRegistry`] keys each spec by its [`AttachmentName`] and answers a
/// lookup — the shape the folder loader (and setup) rely on. A missing key returns [`None`]
/// (fail-closed). Asserts the map mechanism, not any content.
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

// --- the commands-extension apply path ---------------------------------------------------

/// Spawn a distinctive-baseline weapon entity (NOT shipped magnitudes) carrying the stat set
/// the identity / no-op tests touch, and return its id.
fn spawn_weapon(world: &mut World) -> Entity {
    world
        .spawn((
            Accuracy::new(1.0),
            DamageType::Kinetic,
            Magazine::loaded(MagazineSize::new(20), ReloadTu::new(20)),
        ))
        .id()
}

/// Applying NO effects leaves the weapon's stats byte-identical — the identity property (a
/// weapon fitting a cosmetic / empty attachment is unchanged). Exercises the REAL commands
/// path (queue → flush) with an empty list (no `attach_to_weapon` calls).
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

/// An effect whose TARGET component is absent (a mis-seeded weapon) is a fail-safe NO-OP
/// through the commands path — it neither panics nor inserts a component the effect only
/// means to MUTATE. `Aim` reads + re-inserts `Accuracy`, so a weapon with no `Accuracy` gains
/// none.
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

// --- the post-spawn apply_pending_attachments system -------------------------------------

/// Build a minimal app with the system registered and one weapon entity carrying `effects`
/// as a `PendingAttachments` marker (plus a distinctive-baseline stat set the effects target).
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

/// The system applies each pending effect to the weapon (Aim → `Accuracy` raised) AND removes
/// the `PendingAttachments` marker so the apply is one-shot.
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

/// Multiple pending effects each apply (`Silence` inserts `Silenced`, `ExtraAmmo` grows the
/// magazine) — the list is applied in full.
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

/// An EMPTY pending list is the identity — the marker is still removed (one-shot) and no stat
/// changes, so a weapon with no attachments is byte-identical.
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
