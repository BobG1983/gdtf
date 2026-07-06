//! Hot registry editing — an edited attachment spec applies its stronger effect on the NEXT
//! spawn (the registry is the live source of truth the loader rebuilds).

use gdtf_battle_sim::{
    Accuracy,
    effects::attachments::{AimDelta, AttachmentEffect},
    equipment::attachments::{AttachmentName, AttachmentRegistry, AttachmentSlot, AttachmentSpec},
    ganger::Direction,
    test_support::SituationBuilder,
    weapon::WeaponName,
};

use super::harness::*;

// ── Hot-reload: the registry is the LIVE source the loader rebuilds ──────────────

#[test]
fn editing_the_attachment_registry_changes_the_next_spawn() {
    // The loader's hot-reload rebuilds the AttachmentRegistry in place on a `*.attachment.ron`
    // edit (asserted app-side in gdtf_app's resolve::attachments unit tests). Here we prove the
    // registry IS the live source of truth: an edited spec (larger Aim) applies a stronger
    // effect on the next spawn — so a hot-reloaded item takes effect for the weapons spawned
    // after the rebuild.
    let (small_app, small_weapon) =
        spawn_lone_player_weapon(vec![AttachmentEffect::Aim(AimDelta::new(0.2))]);
    let small = small_app.world().get::<Accuracy>(small_weapon).map(|a| **a);

    // Rebuild the same key with a LARGER Aim (the shape of a hot-reload edit): the loader
    // rebuilds this resource in place on a `*.attachment.ron` `Modified` event; here the test
    // body mutates it directly (the accepted headless idiom), then spawns afresh.
    let mut app = battle_app(vec![AttachmentEffect::Aim(AimDelta::new(0.2))]);
    if let Some(mut registry) = app.world_mut().get_resource_mut::<AttachmentRegistry>() {
        registry.insert(
            AttachmentName::new(ATTACHMENT_KEY.to_owned()),
            AttachmentSpec {
                display_name: WeaponName::new("Edited".to_owned()),
                // GTW-554: the edited item keeps the Rail slot the fixture weapon declares.
                slot:         AttachmentSlot::Rail,
                effects:      vec![AttachmentEffect::Aim(AimDelta::new(0.8))],
            },
        );
    }
    let situation = SituationBuilder::new()
        .with_gangers([
            player_at(ground(5, 5), Direction::East),
            enemy_at(ground(20, 20)),
        ])
        .build_with_gangs();
    drive_setup(&mut app, situation);
    let Some(player) = ganger_of(&mut app, PLAYER) else {
        unreachable!("setup spawns one player");
    };
    let Some(weapon) = weapon_entity_of(&mut app, player) else {
        unreachable!("the player wields a ranged weapon entity");
    };
    let edited = app.world().get::<Accuracy>(weapon).map(|a| **a);

    let (Some(small), Some(edited)) = (small, edited) else {
        unreachable!("both weapons carry Accuracy");
    };
    assert!(
        edited > small,
        "an EDITED attachment spec (the hot-reload shape) applies its stronger effect on the \
         next spawn (edited Accuracy {edited} > pre-edit {small}) — the registry is the live \
         source of truth",
    );
}
