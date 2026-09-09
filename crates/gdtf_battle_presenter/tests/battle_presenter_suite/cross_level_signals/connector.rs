use bevy::platform::collections::HashSet;
use gdtf_battle_presenter::{ActiveLevel, CrossLevelBadgeKind, CrossLevelSignals};
use gdtf_battle_sim::{
    prelude::{Cell, CellLevel, Level},
    test_support::SituationBuilder,
    vertical::{LinkKind, VerticalLink, build_vertical_link_graph},
    visibility::SquadVisibility,
};

use super::harness::{settle, signals_app, visible_label_texts, visible_tile_count};

fn key(x: i32, y: i32, z: u8) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(z))
}

#[test]
fn stair_endpoint_on_active_storey_emits_connector_delta() {
    let mut app = signals_app();
    app.world_mut()
        .insert_resource(ActiveLevel::new(Level::new(0)));

    let lower = key(3, 3, 0);
    let upper = key(3, 3, 1);
    let (situation, _placements) = SituationBuilder::new()
        .slab_at(lower)
        .slab_at(upper)
        .vertical_link(VerticalLink::new(lower, upper, LinkKind::stair()))
        .build();
    let result = build_vertical_link_graph(&situation.map);
    assert!(result.is_ok(), "expected a valid graph, got {result:?}");
    let Ok(graph) = result else {
        return;
    };
    app.world_mut().insert_resource(graph);

    let explored: HashSet<CellLevel> = [lower, upper].into_iter().collect();
    app.world_mut()
        .insert_resource(SquadVisibility::new(explored.clone(), explored));

    settle(&mut app);

    let badges: Vec<CrossLevelBadgeKind> = app
        .world()
        .resource::<CrossLevelSignals>()
        .badges_at(lower.cell())
        .to_vec();
    assert!(
        badges.iter().any(|b| matches!(
            *b,
            CrossLevelBadgeKind::ConnectorDelta { delta } if delta.is_above() && delta.magnitude() == 1
        )),
        "the LOWER stair endpoint on the active storey must emit ConnectorDelta(+1) \
         (ascend to the upper endpoint), got {badges:?}",
    );
}

#[test]
fn stair_endpoint_on_active_storey_draws_its_connector_delta_badge() {
    let mut app = signals_app();
    app.world_mut()
        .insert_resource(ActiveLevel::new(Level::new(0)));

    let lower = key(3, 3, 0);
    let upper = key(3, 3, 1);
    let (situation, _placements) = SituationBuilder::new()
        .slab_at(lower)
        .slab_at(upper)
        .vertical_link(VerticalLink::new(lower, upper, LinkKind::stair()))
        .build();
    let result = build_vertical_link_graph(&situation.map);
    assert!(result.is_ok(), "expected a valid graph, got {result:?}");
    let Ok(graph) = result else {
        return;
    };
    app.world_mut().insert_resource(graph);

    let explored: HashSet<CellLevel> = [lower, upper].into_iter().collect();
    app.world_mut()
        .insert_resource(SquadVisibility::new(explored.clone(), explored));

    settle(&mut app);

    assert_eq!(
        visible_tile_count(&mut app),
        1,
        "the sole ConnectorDelta candidate on its cell must be drawn as ONE tile",
    );
    assert_eq!(
        visible_label_texts(&mut app),
        vec!["+1".to_string()],
        "the drawn label must read the ascending ConnectorDelta's sign+magnitude",
    );
}
