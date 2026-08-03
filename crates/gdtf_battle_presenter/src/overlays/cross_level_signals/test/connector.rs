use bevy::platform::collections::HashSet;
use gdtf_battle_sim::{
    prelude::{Cell, CellLevel, Level},
    test_support::SituationBuilder,
    vertical::{LinkKind, VerticalLink, VerticalLinkGraph, build_vertical_link_graph},
    visibility::SquadVisibility,
};

use crate::overlays::cross_level_signals::{connector::gather_connectors, types::LevelDelta};

fn key(x: i32, y: i32, z: u8) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(z))
}

fn stair_graph(lower: CellLevel, upper: CellLevel) -> Option<VerticalLinkGraph> {
    let situation = SituationBuilder::new()
        .slab_at(lower)
        .slab_at(upper)
        .vertical_link(VerticalLink::new(lower, upper, LinkKind::stair()))
        .build();
    let result = build_vertical_link_graph(&situation);
    assert!(result.is_ok(), "expected a valid graph, got {result:?}");
    result.ok()
}

#[test]
fn lower_endpoint_on_active_storey_emits_ascending_delta() {
    let lower = key(3, 3, 0);
    let upper = key(3, 3, 1);
    let Some(graph) = stair_graph(lower, upper) else {
        return;
    };
    let explored: HashSet<CellLevel> = [lower, upper].into_iter().collect();
    let squad = SquadVisibility::new(explored.clone(), explored);

    let connectors = gather_connectors(Level::new(0), &graph, &squad);

    assert_eq!(
        connectors,
        vec![(lower.cell(), LevelDelta::new(1))],
        "the LOWER stair endpoint on the active storey emits ConnectorDelta(+1) \
         (ascend to the upper endpoint)",
    );
}

#[test]
fn upper_endpoint_on_active_storey_emits_descending_delta() {
    let lower = key(3, 3, 0);
    let upper = key(3, 3, 1);
    let Some(graph) = stair_graph(lower, upper) else {
        return;
    };
    let explored: HashSet<CellLevel> = [lower, upper].into_iter().collect();
    let squad = SquadVisibility::new(explored.clone(), explored);

    let connectors = gather_connectors(Level::new(1), &graph, &squad);

    assert_eq!(
        connectors,
        vec![(upper.cell(), LevelDelta::new(-1))],
        "the UPPER stair endpoint on the active storey emits ConnectorDelta(-1) \
         (descend to the lower endpoint)",
    );
}

#[test]
fn unexplored_endpoint_emits_no_connector() {
    let lower = key(3, 3, 0);
    let upper = key(3, 3, 1);
    let Some(graph) = stair_graph(lower, upper) else {
        return;
    };
    let squad = SquadVisibility::default();

    let connectors = gather_connectors(Level::new(0), &graph, &squad);

    assert!(
        connectors.is_empty(),
        "an unexplored link endpoint never surfaces a connector badge",
    );
}
