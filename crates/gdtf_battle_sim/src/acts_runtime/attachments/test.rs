//! Tests for [`apply_pending_attachments`] — the post-spawn attachment-application system
//! that drains a weapon's [`PendingAttachments`](crate::weapon::PendingAttachments) marker and
//! applies each effect via the commands extension, then removes the marker (one-shot).
//!
//! These drive the REAL system through a minimal [`App`]: spawn a weapon entity carrying a
//! `PendingAttachments` marker, `app.update()` (running the system + flushing its queued
//! commands), and assert (a) the effect mutated the correct component and (b) the marker was
//! removed so a second update is a no-op. Value-agnostic on magnitudes (mapping + direction
//! only), per the brittle-test rule.

use bevy::prelude::{App, Entity, Update};

use super::apply_pending_attachments;
use crate::{
    magazine::{Magazine, ReloadTu},
    weapon::{
        Accuracy, AimDelta, AttachmentEffect, MagazineSize, PendingAttachments, Silenced,
        WeaponBraceBonus,
    },
};

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
            .get::<WeaponBraceBonus>()
            .is_none(),
        "an empty pending list inserts no brace bonus",
    );
    assert!(
        app.world()
            .entity(weapon)
            .get::<PendingAttachments>()
            .is_none(),
        "the (empty) PendingAttachments marker is still removed (one-shot)",
    );
}
