use gdtf_battle_sim::{
    effects::attachments::{AimDelta, AttachmentEffect},
    equipment::attachments::{AttachmentName, AttachmentRegistry, AttachmentSlot, AttachmentSpec},
    ganger::Direction,
    test_support::SituationBuilder,
    weapon::{Accuracy, WeaponName},
};

use super::harness::*;


#[test]
fn editing_the_attachment_registry_changes_the_next_spawn() {
    let (small_app, small_weapon) =
        spawn_lone_player_weapon(vec![AttachmentEffect::Aim(AimDelta::new(0.2))]);
    let small = small_app.world().get::<Accuracy>(small_weapon).map(|a| **a);

    let mut app = battle_app(vec![AttachmentEffect::Aim(AimDelta::new(0.2))]);
    if let Some(mut registry) = app.world_mut().get_resource_mut::<AttachmentRegistry>() {
        registry.insert(
            AttachmentName::new(ATTACHMENT_KEY.to_owned()),
            AttachmentSpec {
                display_name: WeaponName::new("Edited".to_owned()),
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
