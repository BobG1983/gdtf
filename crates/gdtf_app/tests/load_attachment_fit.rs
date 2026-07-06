//! GTW-549 / GTW-554: the SHIPPED weapon↔attachment reference + slot-fit pins
//! — the family-bespoke half of the attachments load coverage (the generic
//! tier + deep-behavior coverage lives in `load_attachments.rs`; this file is
//! NEVER genericized away, the GTW-580 P9 rule).
//!
//! Tier (b) — `DefaultPlugins` (headless, `backends: None`) via
//! [`GdtfLoadTestAppBuilder`]: a real `AssetServer` pointed at the workspace
//! `assets/`. Proves the SHIPPED attachment items PARSE from RON through the
//! real folder-load path, that the KEYS authored on the shipped weapons
//! resolve to loaded specs with non-empty effect lists, and that each fitted
//! item's slot FITS the weapon's declared slots (the GTW-554 gate).
//!
//! Pin-discriminating + VALUE-AGNOSTIC: presence / non-emptiness / fit only —
//! never a shipped magnitude (the brittle-test rule), so a tuning edit never
//! reddens it but a malformed shipped RON, a variant-name typo, a stem-strip
//! regression, or a bad weapon slot key does. The effect-to-stat application
//! MECHANISM is covered by the sim apply/spawn tests.

use gdtf_app::test_support::{AppState, app_state};
use gdtf_battle_sim::weapon::{
    AttachmentName, AttachmentRegistry, MeleeWeaponRegistry, WeaponName, WeaponRegistry,
    attachment_fits,
};
use gdtf_test_utils::{GdtfLoadTestAppBuilder, advance_until_resource_exists};

/// Generous SAFETY-NET cap for the real-asset `advance_until_resource_exists` waits gated on
/// the async folder loads resolving. The loads share the `AssetServer` with the full scene
/// stack, so under parallel `cargo` contention the async resolve has NO fixed frame count;
/// the wait keys off the resolved SIGNAL and the cap is a safety net against a genuine
/// never-resolve hang, not a timing budget (GTW-305).
const LOAD_SAFETY_NET: u32 = 10_000;

/// GTW-549 — with a real `AssetServer` rooted at the workspace `assets/`, entering `Load`
/// builds an [`AttachmentRegistry`] keyed by each file's stem, and the KEYS the shipped
/// weapons authored (`scoped_sight` on `las_carbine`, `suppressor` on `stub_pistol`) resolve
/// — both to the loaded attachment spec AND, transitively, to a NON-EMPTY effect list (so
/// the live weapon references don't resolve to nothing at runtime). GTW-554: each shipped
/// weapon's declared `slots:` must FIT the item it authors.
#[test]
fn shipped_weapon_attachment_keys_resolve_and_fit() {
    let mut app = GdtfLoadTestAppBuilder::new()
        .starting_in(AppState::Load)
        .build();

    // Signal-poll the async folder loads: the resolvers only insert each registry ONCE
    // fully built (never partial), so on the good path existence implies the shipped
    // content parsed. Caps are safety nets (GTW-305).
    advance_until_resource_exists::<AttachmentRegistry>(&mut app, LOAD_SAFETY_NET);
    advance_until_resource_exists::<WeaponRegistry>(&mut app, LOAD_SAFETY_NET);
    advance_until_resource_exists::<MeleeWeaponRegistry>(&mut app, LOAD_SAFETY_NET);

    // Assert PRESENCE loudly first (pin-discriminating), THEN bind without a panic
    // (restriction lints deny `panic!` even in tests).
    assert!(
        app.world().get_resource::<AttachmentRegistry>().is_some()
            && app.world().get_resource::<WeaponRegistry>().is_some(),
        "both an AttachmentRegistry and a WeaponRegistry must resolve from the workspace \
         assets/ (last AppState was {:?})",
        app_state(&app),
    );
    let (Some(attachments), Some(weapons)) = (
        app.world().get_resource::<AttachmentRegistry>(),
        app.world().get_resource::<WeaponRegistry>(),
    ) else {
        return;
    };

    // The shipped attachment items PARSED and populated the registry (the folder-loaded
    // parse clause) — a malformed / variant-typo'd shipped RON would leave this empty.
    assert!(
        !attachments.is_empty(),
        "the resolved AttachmentRegistry must carry the shipped (non-empty) attachment items \
         — a bad/malformed shipped `*.attachment.ron` would leave it empty",
    );

    for (weapon_key, attachment_key) in [
        ("las_carbine", "scoped_sight"),
        ("stub_pistol", "suppressor"),
    ] {
        let name = AttachmentName::new(attachment_key.to_owned());

        // The shipped weapon RON references the attachment slot KEY.
        let weapon_authors_key = weapons
            .spec(&WeaponName::new(weapon_key.to_owned()))
            .is_some_and(|weapon| weapon.attachments.contains(&name));
        assert!(
            weapon_authors_key,
            "the shipped weapon `{weapon_key}` must resolve and author the `{attachment_key}` \
             attachment slot key (the shipped weapon RON references it)",
        );

        // The key resolves to a loaded attachment spec with a NON-EMPTY effect list — a
        // stem-strip regression, a missing shipped file, or an effect-less item breaks this.
        let key_resolves_to_effects = attachments
            .spec(&name)
            .is_some_and(|spec| !spec.effects.is_empty());
        assert!(
            key_resolves_to_effects,
            "the `{attachment_key}` key authored on `{weapon_key}` must resolve to a loaded \
             attachment spec with a non-empty effect list (not resolve to nothing at runtime)",
        );

        // GTW-554: the shipped weapon's declared `slots:` must FIT the item it authors — a
        // content mistake (fitted item whose slot the weapon never declares) would make the
        // live reference resolve to nothing under the slot gate. Value-agnostic: it asserts
        // the FIT, never a capacity magnitude.
        let item_fits = match (
            weapons.spec(&WeaponName::new(weapon_key.to_owned())),
            attachments.spec(&name),
        ) {
            (Some(weapon), Some(item)) => attachment_fits(&weapon.slots, &[], item.slot).is_ok(),
            _ => false,
        };
        assert!(
            item_fits,
            "the shipped `{weapon_key}` must declare a slot the fitted `{attachment_key}` \
             occupies (the GTW-554 slot gate would otherwise cleanly reject the live fitting)",
        );
    }

    // GTW-554: the shipped MELEE pair — `chainsword` fits the `butchers_weight` counterweight
    // through the same slot gate (melee weapons gained FULL attachment support).
    let Some(melee) = app.world().get_resource::<MeleeWeaponRegistry>() else {
        unreachable!("the MeleeWeaponRegistry resolved with the Load gate");
    };
    let weight = AttachmentName::new("butchers_weight".to_owned());
    let melee_pair_fits = match (
        melee.spec(&WeaponName::new("chainsword".to_owned())),
        attachments.spec(&weight),
    ) {
        (Some(sword), Some(item)) => {
            sword.attachments.contains(&weight)
                && attachment_fits(&sword.slots, &[], item.slot).is_ok()
        }
        _ => false,
    };
    assert!(
        melee_pair_fits,
        "the shipped `chainsword` must author the `butchers_weight` key AND declare the \
         Counterweight slot it occupies (the GTW-554 melee attachment support, live)",
    );
}
