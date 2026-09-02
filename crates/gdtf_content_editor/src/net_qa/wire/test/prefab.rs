use gdtf_battle_sim::level::SpawnRole;

use super::{assert_ron_round_trip, assert_schema_is_usable};
use crate::net_qa::wire::{PrefabPlacementCountNet, SpawnRoleNet};

#[test]
fn every_spawn_role_round_trips() {
    for role in SpawnRoleNet::ALL {
        assert_ron_round_trip(&role);
    }
}

#[test]
fn each_spawn_role_reads_back_as_its_own_sim_role() {
    let read: Vec<SpawnRole> = SpawnRoleNet::ALL
        .into_iter()
        .map(SpawnRoleNet::to_role)
        .collect();
    assert_eq!(
        read,
        vec![SpawnRole::Player, SpawnRole::Enemy, SpawnRole::Fill],
        "the wire role is the half of a PrefabKey a client picks, so a swapped pair would open \
         the enemy's deployment area when the player's was asked for",
    );
}

#[test]
fn a_placement_count_round_trips() {
    assert_ron_round_trip(&PrefabPlacementCountNet::new(7));
}

#[test]
fn a_placement_count_carries_the_cell_count_it_wrapped() {
    assert_eq!(
        *PrefabPlacementCountNet::new(7),
        7,
        "the count a client reads back is the number of cells the open painted",
    );
}

#[test]
fn the_prefab_types_trace_usable_shapes() {
    assert_schema_is_usable::<SpawnRoleNet>("SpawnRoleNet");
    assert_schema_is_usable::<PrefabPlacementCountNet>("PrefabPlacementCountNet");
}
