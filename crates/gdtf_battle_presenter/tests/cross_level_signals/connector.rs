//! `ConnectorDelta` badges: the positive case proven through the REAL registered
//! derive system reading a live `VerticalLinkGraph` (the sibling in-crate
//! `test/connector.rs` pins the pure `gather_connectors` helper directly).

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

/// Acceptance clause 2 (`ConnectorDelta`): the LOWER stair endpoint on the active
/// storey emits an ascending `ConnectorDelta(+1)` badge — asserted on the REAL
/// `CrossLevelSignals` resource the registered derive system writes, over a
/// `VerticalLinkGraph` built through the real `build_vertical_link_graph`.
#[test]
fn stair_endpoint_on_active_storey_emits_connector_delta() {
    let mut app = signals_app();
    app.world_mut()
        .insert_resource(ActiveLevel::new(Level::new(0)));

    let lower = key(3, 3, 0);
    let upper = key(3, 3, 1);
    let situation = SituationBuilder::new()
        .slab_at(lower)
        .slab_at(upper)
        .vertical_link(VerticalLink::new(lower, upper, LinkKind::stair()))
        .build();
    let result = build_vertical_link_graph(&situation);
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

/// Gate finding (GTW-596 QA lens): every OTHER acceptance-clause integration test
/// that includes a `ConnectorDelta` candidate happens to have it LOSE the
/// 3-badge cap (`cap.rs`'s dropped `"+1"`), so nothing yet proved a KEPT
/// `ConnectorDelta` badge actually reaches the drawn pool. This is the sole
/// candidate on its cell (no Threat / `DropDepth` competing), so it trivially
/// wins the cap — asserted DRAWN: exactly one visible tile, exactly one visible
/// label reading `"+1"`.
#[test]
fn stair_endpoint_on_active_storey_draws_its_connector_delta_badge() {
    let mut app = signals_app();
    app.world_mut()
        .insert_resource(ActiveLevel::new(Level::new(0)));

    let lower = key(3, 3, 0);
    let upper = key(3, 3, 1);
    let situation = SituationBuilder::new()
        .slab_at(lower)
        .slab_at(upper)
        .vertical_link(VerticalLink::new(lower, upper, LinkKind::stair()))
        .build();
    let result = build_vertical_link_graph(&situation);
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
