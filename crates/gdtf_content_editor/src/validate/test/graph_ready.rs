//! The Check window stays shut until every registry its checks read exists.

use bevy::ecs::{system::RunSystemOnce as _, world::World};
use gdtf_battle_sim::{
    armor::ArmorRegistry,
    effects::fields::FieldDefRegistry,
    equipment::attachments::AttachmentRegistry,
    ganger::GangRegistry,
    injuries::InjuryRegistry,
    level::{PrefabRegistry, UuidThemeRegistry},
    situation::Situation,
    terrain::def::TerrainDefRegistry,
    weapon::{MeleeWeaponRegistry, WeaponRegistry},
};
use gdtf_content_families::{situation::LoadedSituation, sprites::SpriteDefRegistry};

use crate::validate::register::validation_graph_ready;

// Every resource the gate names but the registries the caller withholds.
fn world_holding_everything_else(withheld: Withheld) -> World {
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
    if withheld != Withheld::Prefabs {
        world.init_resource::<PrefabRegistry>();
    }
    if withheld != Withheld::Fields {
        world.init_resource::<FieldDefRegistry>();
    }
    if withheld != Withheld::Situation {
        world.insert_resource(LoadedSituation::new(Situation::default()));
    }
    world
}

// The one resource a case keeps out of its world.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Withheld {
    Prefabs,
    Fields,
    Situation,
}

// Run the gate over a world missing one resource, answering what it said.
fn ready_without(withheld: Withheld) -> Option<bool> {
    let mut world = world_holding_everything_else(withheld);
    let answer = world.run_system_once(validation_graph_ready);
    assert!(
        answer.is_ok(),
        "running the validation-graph condition must succeed: {:?}",
        answer.as_ref().err(),
    );
    answer.ok()
}

#[test]
fn the_validation_graph_is_not_ready_without_the_prefab_registry() {
    let Some(ready) = ready_without(Withheld::Prefabs) else {
        return;
    };
    assert!(
        !ready,
        "the graph must NOT be ready while PrefabRegistry is absent — every other watched \
         registry is in this world, so nothing else can hold the condition false",
    );
}

#[test]
fn the_validation_graph_is_not_ready_without_the_loaded_situation() {
    let Some(ready) = ready_without(Withheld::Situation) else {
        return;
    };
    assert!(
        !ready,
        "the four situation checks read LoadedSituation, and a check whose Res is missing is \
         skipped rather than reddened, so the Check window must stay shut without it",
    );
}

#[test]
fn the_validation_graph_is_not_ready_without_the_field_def_registry() {
    let Some(ready) = ready_without(Withheld::Fields) else {
        return;
    };
    assert!(
        !ready,
        "the situation's field-key check reads FieldDefRegistry, so the Check window must stay \
         shut until it exists",
    );
}
