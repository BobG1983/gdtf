//! real folder-load path, that the KEYS authored on the shipped weapons
use cobalt_test_utils::{LoadTestAppBuilder, advance_until_resource_exists};
use gdtf_battle_sim::{
    equipment::attachments::{AttachmentName, AttachmentRegistry, AttachmentSlot, attachment_fits},
    weapon::{MeleeWeaponRegistry, WeaponName, WeaponRegistry},
};
use gdtf_game::test_support::{AppState, app_state};

#[test]
fn shipped_weapon_attachment_keys_resolve_and_fit() {
    let mut app =
        LoadTestAppBuilder::new(gdtf_game::test_support::register_scenes_with_default_plugins)
            .starting_in(AppState::Load)
            .build();

    advance_until_resource_exists::<AttachmentRegistry>(&mut app);
    advance_until_resource_exists::<WeaponRegistry>(&mut app);
    advance_until_resource_exists::<MeleeWeaponRegistry>(&mut app);

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

        let weapon_authors_key = weapons
            .spec(&WeaponName::new(weapon_key.to_owned()))
            .is_some_and(|weapon| weapon.attachments.contains(&name));
        assert!(
            weapon_authors_key,
            "the shipped weapon `{weapon_key}` must resolve and author the `{attachment_key}` \
             attachment slot key (the shipped weapon RON references it)",
        );

        let key_resolves_to_effects = attachments
            .spec(&name)
            .is_some_and(|spec| !spec.effects.is_empty());
        assert!(
            key_resolves_to_effects,
            "the `{attachment_key}` key authored on `{weapon_key}` must resolve to a loaded \
             attachment spec with a non-empty effect list (not resolve to nothing at runtime)",
        );

        let item_fits = match (
            weapons.spec(&WeaponName::new(weapon_key.to_owned())),
            attachments.spec(&name),
        ) {
            (Some(weapon), Some(item)) => attachment_fits(&weapon.slots, &[], item.slot).is_ok(),
            _ => false,
        };
        assert!(
            item_fits,
            "the shipped `{weapon_key}` must declare a slot the fitted `{attachment_key}` occupies",
        );
    }

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
        "the shipped `chainsword` must author the `butchers_weight` key AND declare the Counterweight slot it occupies",
    );

    assert_magazine_slot_fits_and_rejects(attachments, weapons, melee);
}

fn assert_magazine_slot_fits_and_rejects(
    attachments: &AttachmentRegistry,
    weapons: &WeaponRegistry,
    melee: &MeleeWeaponRegistry,
) {
    let Some(mag_item) = attachments.spec(&AttachmentName::new("extended_mag".to_owned())) else {
        unreachable!("the shipped `extended_mag` attachment must resolve from assets/");
    };
    assert_eq!(
        mag_item.slot,
        AttachmentSlot::Magazine,
        "the shipped `extended_mag` must be re-slotted onto the Magazine well",
    );

    let mag_fits_ranged = weapons
        .spec(&WeaponName::new("stub_pistol".to_owned()))
        .is_some_and(|gun| attachment_fits(&gun.slots, &[], mag_item.slot).is_ok());
    assert!(
        mag_fits_ranged,
        "the magazine-fed `stub_pistol` must declare a Magazine slot the `extended_mag` fits",
    );

    let mag_rejected_by_melee = melee
        .spec(&WeaponName::new("chainsword".to_owned()))
        .is_some_and(|sword| attachment_fits(&sword.slots, &[], mag_item.slot).is_err());
    assert!(
        mag_rejected_by_melee,
        "the melee `chainsword` offers no Magazine slot, so the `extended_mag` is cleanly rejected",
    );
}
