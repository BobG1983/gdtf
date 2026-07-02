//! Unit tests for the GTW-549 PHASE 2 **effect-isolation architecture**: each
//! [`AttachmentEffect`](super::AttachmentEffect) variant, applied to a real weapon entity
//! through the [`attach_to_weapon`](super::AttachToWeaponExt::attach_to_weapon) commands
//! extension, mutates the CORRECT stat / component; an empty effect list is the identity;
//! and an effect whose target component is absent is a fail-safe no-op.
//!
//! These exercise the REAL application path: a weapon entity is spawned in a fresh
//! [`World`], the effect is queued via `world.commands().attach_to_weapon(..)`, the buffer
//! is FLUSHED (running the deferred [`EntityCommand`](bevy::ecs::system::EntityCommand)),
//! and the resulting component is asserted. Per the brittle-test rule the assertions check
//! the MAPPING + DIRECTION (which stat, raised / inserted / overridden) against a distinctive
//! baseline — never a shipped magnitude.

use bevy::prelude::{Entity, World};

use super::{
    AttachToWeaponExt, AttachmentEffect,
    apply::ApplyAttachmentEffect,
    magnitude::{AimDelta, ReloadScale, WeaponBraceBonus},
};
use crate::{
    magazine::{Magazine, ReloadTu},
    weapon::{
        Accuracy, DamageType, FatalBias, FireMode, FireModeSpec, MagazineSize, ModeConeMult,
        ModeKind, ModeShots, ModeTuPercent, Shove, Silenced, Stable, WeaponDamage, WeaponPunch,
        WeaponShred,
    },
};

/// Spawn a distinctive-baseline weapon entity (NOT shipped magnitudes) carrying the full
/// stat set an effect might target, and return its id.
fn spawn_weapon(world: &mut World) -> Entity {
    world
        .spawn((
            Accuracy::new(1.0),
            WeaponPunch::new(4),
            WeaponDamage::new(10),
            WeaponShred::new(2),
            FatalBias::new(2.0),
            DamageType::Kinetic,
            Magazine::loaded(MagazineSize::new(20), ReloadTu::new(20)),
            FireMode::new(vec![FireModeSpec::new(
                ModeKind::Single,
                ModeConeMult::new(1.0),
                ModeTuPercent::new(0.2),
                ModeShots::new(1),
            )]),
        ))
        .id()
}

/// Apply one effect to a freshly-spawned weapon entity through the REAL commands-extension
/// path (queue → flush) and return the mutated `World` + the weapon id for assertion.
fn apply(effect: AttachmentEffect) -> (World, Entity) {
    let mut world = World::new();
    let weapon = spawn_weapon(&mut world);
    world.commands().attach_to_weapon(weapon, effect);
    world.flush();
    (world, weapon)
}

/// `Aim` raises the weapon's `Accuracy` — the HEADLINE GTW-549 fix (a sight boosts AIM, the
/// in-cone concentration exponent, NOT stability). Asserts the mapping + direction.
#[test]
fn aim_raises_accuracy() {
    let (world, weapon) = apply(AttachmentEffect::Aim(AimDelta::new(0.4)));
    let Some(accuracy) = world.entity(weapon).get::<Accuracy>() else {
        unreachable!("the weapon must still carry Accuracy");
    };
    assert!(
        **accuracy > 1.0,
        "Aim raises Accuracy above the 1.0 baseline (got {})",
        **accuracy
    );
}

/// `Stability` INSERTS a graduated `WeaponBraceBonus` — the NEW clean brace seam (NOT the
/// ripped-out sight-stability). Asserts the component is present with the authored magnitude.
#[test]
fn stability_inserts_weapon_brace_bonus() {
    let (world, weapon) = apply(AttachmentEffect::Stability(WeaponBraceBonus::new(12.0)));
    let Some(bonus) = world.entity(weapon).get::<WeaponBraceBonus>() else {
        unreachable!("Stability must insert a WeaponBraceBonus component");
    };
    assert!(
        **bonus > 0.0,
        "Stability inserts a positive brace bonus (got {})",
        **bonus
    );
}

/// `GainFireMode` ADDS one `FireModeSpec` to the selector — the count grows by exactly one.
#[test]
fn gain_fire_mode_appends_a_mode() {
    let mut world = World::new();
    let weapon = spawn_weapon(&mut world);
    let before = world
        .entity(weapon)
        .get::<FireMode>()
        .map_or(0, |m| m.len());
    let burst = FireModeSpec::new(
        ModeKind::Burst,
        ModeConeMult::new(1.3),
        ModeTuPercent::new(0.5),
        ModeShots::new(3),
    );
    world
        .commands()
        .attach_to_weapon(weapon, AttachmentEffect::GainFireMode(burst));
    world.flush();
    let after = world
        .entity(weapon)
        .get::<FireMode>()
        .map_or(0, |m| m.len());
    assert_eq!(after, before + 1, "GainFireMode appends exactly one mode");
}

/// `ExtraAmmo` GROWS the magazine capacity and refills it to the new full.
#[test]
fn extra_ammo_grows_the_magazine() {
    let (world, weapon) = apply(AttachmentEffect::ExtraAmmo(MagazineSize::new(6)));
    let Some(magazine) = world.entity(weapon).get::<Magazine>() else {
        unreachable!("the weapon must still carry a Magazine");
    };
    assert!(
        magazine.size().get() > 20,
        "ExtraAmmo grows capacity above the 20 baseline (got {})",
        magazine.size().get()
    );
    assert!(
        magazine.is_full(),
        "ExtraAmmo refills the grown magazine to full"
    );
}

/// `FastReload` LOWERS the magazine's reload cost (its per-item scale, `< 1.0`).
#[test]
fn fast_reload_lowers_reload_tu() {
    let (world, weapon) = apply(AttachmentEffect::FastReload(ReloadScale::new(0.5)));
    let Some(magazine) = world.entity(weapon).get::<Magazine>() else {
        unreachable!("the weapon must still carry a Magazine");
    };
    assert!(
        *magazine.reload_tu() < 20,
        "FastReload lowers reload_tu below the 20 baseline (got {})",
        *magazine.reload_tu()
    );
}

/// `Silence` INSERTS the `Silenced(true)` tag — preserving the component both producer gates
/// read.
#[test]
fn silence_inserts_silenced_tag() {
    let (world, weapon) = apply(AttachmentEffect::Silence);
    let Some(silenced) = world.entity(weapon).get::<Silenced>() else {
        unreachable!("Silence must insert a Silenced component");
    };
    assert!(**silenced, "Silence fits Silenced(true)");
}

/// `Penetration` RAISES the weapon's `WeaponPunch` (additive).
#[test]
fn penetration_raises_punch() {
    let (world, weapon) = apply(AttachmentEffect::Penetration(WeaponPunch::new(5)));
    let Some(punch) = world.entity(weapon).get::<WeaponPunch>() else {
        unreachable!("the weapon must still carry WeaponPunch");
    };
    assert!(
        **punch > 4,
        "Penetration raises punch above the 4 baseline (got {})",
        **punch
    );
}

/// `DamageTypeOverride` REPLACES the weapon's emitted `DamageType`.
#[test]
fn damage_type_override_replaces_the_type() {
    let (world, weapon) = apply(AttachmentEffect::DamageTypeOverride(DamageType::Chem));
    let Some(damage_type) = world.entity(weapon).get::<DamageType>() else {
        unreachable!("the weapon must still carry a DamageType");
    };
    assert_eq!(
        *damage_type,
        DamageType::Chem,
        "DamageTypeOverride replaces Kinetic with the authored Chem"
    );
}

/// `Damage` RAISES base damage; `Shred` RAISES shred; `FatalBias` RAISES fatal bias — the
/// USER-REVIEW additive extras all land on their own stat.
#[test]
fn additive_extras_raise_their_own_stat() {
    let (world, weapon) = apply(AttachmentEffect::Damage(WeaponDamage::new(5)));
    let Some(damage) = world.entity(weapon).get::<WeaponDamage>() else {
        unreachable!("WeaponDamage present");
    };
    assert!(**damage > 10, "Damage raises base damage above 10");

    let (world, weapon) = apply(AttachmentEffect::Shred(WeaponShred::new(3)));
    let Some(shred) = world.entity(weapon).get::<WeaponShred>() else {
        unreachable!("WeaponShred present");
    };
    assert!(**shred > 2, "Shred raises shred above 2");

    let (world, weapon) = apply(AttachmentEffect::FatalBias(FatalBias::new(1.5)));
    let Some(bias) = world.entity(weapon).get::<FatalBias>() else {
        unreachable!("FatalBias present");
    };
    assert!(**bias > 2.0, "FatalBias raises fatal bias above 2.0");
}

/// `Brace` fits `Stable(true)`; `Shove` fits `Shove(true)` — the no-payload tag extras.
#[test]
fn brace_and_shove_insert_their_boolean_tags() {
    let (world, weapon) = apply(AttachmentEffect::Brace);
    let Some(stable) = world.entity(weapon).get::<Stable>() else {
        unreachable!("Brace must insert Stable");
    };
    assert!(**stable, "Brace fits Stable(true)");

    let (world, weapon) = apply(AttachmentEffect::Shove);
    let Some(shove) = world.entity(weapon).get::<Shove>() else {
        unreachable!("Shove must insert Shove");
    };
    assert!(**shove, "Shove fits Shove(true)");
}

/// Applying NO effects leaves the weapon's stats byte-identical — the identity property (a
/// weapon fitting a cosmetic / empty attachment is unchanged).
#[test]
fn empty_effect_list_is_the_identity() {
    let mut world = World::new();
    let weapon = spawn_weapon(&mut world);
    let before_acc = world.entity(weapon).get::<Accuracy>().copied();
    let before_punch = world.entity(weapon).get::<WeaponPunch>().copied();
    let before_mag = world.entity(weapon).get::<Magazine>().copied();

    // An empty list applies nothing — no attach_to_weapon calls.
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
        world.entity(weapon).get::<WeaponPunch>().copied(),
        before_punch,
        "no effects leaves WeaponPunch unchanged"
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

/// An effect whose TARGET component is absent (a mis-seeded weapon) is a fail-safe NO-OP —
/// it neither panics nor inserts a component the effect only means to MUTATE. `Aim` reads +
/// re-inserts `Accuracy`, so a weapon with no `Accuracy` gains none.
#[test]
fn absent_target_component_is_a_noop() {
    let mut world = World::new();
    // A bare entity with NO Accuracy — Aim reads-then-reinserts, so it must add nothing.
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

/// The isolated behaviour type can also be applied DIRECTLY through the trait against an
/// `EntityWorldMut` (the seam the commands extension wraps) — proving the effect logic lives
/// in the isolated type, not the extension.
#[test]
fn isolated_type_applies_through_the_trait() {
    use super::apply::ApplyAim;
    let mut world = World::new();
    let weapon = spawn_weapon(&mut world);
    let mut entity = world.entity_mut(weapon);
    ApplyAim::new(AimDelta::new(0.5)).apply_to_weapon(&mut entity);
    let Some(accuracy) = entity.get::<Accuracy>() else {
        unreachable!("Accuracy present");
    };
    assert!(
        **accuracy > 1.0,
        "the isolated ApplyAim type raises Accuracy through the trait method directly"
    );
}
