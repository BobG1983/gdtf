//! The Check window stays shut until every registry its checks read exists.

use bevy::ecs::{system::RunSystemOnce as _, world::World};
use gdtf_battle_sim::{
    armor::ArmorRegistry,
    equipment::attachments::AttachmentRegistry,
    ganger::GangRegistry,
    injuries::InjuryRegistry,
    level::UuidThemeRegistry,
    terrain::def::TerrainDefRegistry,
    weapon::{MeleeWeaponRegistry, WeaponRegistry},
};
use gdtf_content_families::sprites::SpriteDefRegistry;

use crate::validate::register::validation_graph_ready;

#[test]
fn the_validation_graph_is_not_ready_without_the_prefab_registry() {
    let mut world = World::new();
    world.init_resource::<WeaponRegistry>();
    world.init_resource::<MeleeWeaponRegistry>();
    world.init_resource::<ArmorRegistry>();
    world.init_resource::<GangRegistry>();
    world.init_resource::<TerrainDefRegistry>();
    world.init_resource::<UuidThemeRegistry>();
    world.init_resource::<InjuryRegistry>();
    world.init_resource::<SpriteDefRegistry>();
    world.init_resource::<AttachmentRegistry>();

    let answer = world.run_system_once(validation_graph_ready);
    assert!(
        answer.is_ok(),
        "running the validation-graph condition must succeed: {:?}",
        answer.as_ref().err(),
    );
    let Ok(ready) = answer else { return };

    assert!(
        !ready,
        "the graph must NOT be ready while PrefabRegistry is absent — every other watched \
         registry is in this world, so nothing else can hold the condition false",
    );
}
