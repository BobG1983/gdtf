use super::{
    InvalidVerticalLink, LinkKind, OneWay, VerticalLink, VerticalLinkGraph,
    build_vertical_link_graph, traversable_links,
};
use crate::{
    ganger::Tu,
    metric::{CellLevel, MAX_LEVELS},
    situation::Situation,
    test_support::{SituationBuilder, key},
    tuning::LinkTu,
};

fn situation_with(cells: &[CellLevel], links: Vec<VerticalLink>) -> Situation {
    let mut builder = SituationBuilder::new();
    for &cell in cells {
        builder = builder.slab_at(cell);
    }
    for link in links {
        builder = builder.vertical_link(link);
    }
    builder.build()
}

fn ok_graph(situation: &Situation) -> Option<VerticalLinkGraph> {
    let result = build_vertical_link_graph(situation);
    assert!(result.is_ok(), "expected a valid graph, got {result:?}");
    result.ok()
}

#[test]
fn valid_links_build_and_query_both_directions() {
    let lower = key(5, 5, 0);
    let upper = key(5, 5, 1);
    let link = VerticalLink::new(lower, upper, LinkKind::stair());
    let situation = situation_with(&[lower, upper], vec![link]);

    let Some(graph) = ok_graph(&situation) else {
        return;
    };

    assert_eq!(graph.len(), 1, "one authored link");

    let from_lower: Vec<_> = graph.links_from(&lower).collect();
    assert_eq!(
        from_lower,
        vec![&link],
        "the link departs the lower endpoint"
    );

    let from_upper: Vec<_> = graph.links_from(&upper).collect();
    assert_eq!(
        from_upper,
        vec![&link],
        "a bidirectional link departs the upper endpoint too (reverse direction)",
    );

    assert_eq!(graph.links_from(&key(9, 9, 0)).count(), 0);
}

#[test]
fn one_way_link_is_forward_only() {
    let lower = key(2, 3, 0);
    let upper = key(2, 3, 1);
    let link = VerticalLink::new(
        lower,
        upper,
        LinkKind::Stair {
            one_way: OneWay::forward_only(),
        },
    );
    let situation = situation_with(&[lower, upper], vec![link]);

    let Some(graph) = ok_graph(&situation) else {
        return;
    };

    assert_eq!(
        graph.links_from(&lower).collect::<Vec<_>>(),
        vec![&link],
        "a one-way link departs its `from` endpoint",
    );
    assert_eq!(
        graph.links_from(&upper).count(),
        0,
        "a one-way link is NOT traversable from its `to` endpoint",
    );
}

#[test]
fn level_at_max_levels_is_out_of_range() {
    let bad = key(1, 1, MAX_LEVELS);
    let ok = key(1, 1, 0);
    let link = VerticalLink::new(bad, ok, LinkKind::ladder());
    let situation = situation_with(&[bad, ok], vec![link]);

    let result = build_vertical_link_graph(&situation);
    assert_eq!(
        result.err(),
        Some(InvalidVerticalLink::LevelOutOfRange { link }),
        "level_from = MAX_LEVELS must be rejected as out of range",
    );
}

#[test]
fn dangling_endpoint_cell_is_rejected() {
    let present = key(4, 4, 0);
    let missing = key(4, 4, 1);
    let link = VerticalLink::new(present, missing, LinkKind::stair());
    let situation = situation_with(&[present], vec![link]);

    let result = build_vertical_link_graph(&situation);
    assert_eq!(
        result.err(),
        Some(InvalidVerticalLink::DanglingCell { link }),
        "an endpoint cell absent from authored cells must be rejected as dangling",
    );
}

#[test]
fn same_level_link_is_rejected() {
    let a = key(7, 7, 2);
    let b = key(8, 8, 2);
    let link = VerticalLink::new(a, b, LinkKind::stair());
    let situation = situation_with(&[a, b], vec![link]);

    let result = build_vertical_link_graph(&situation);
    assert_eq!(
        result.err(),
        Some(InvalidVerticalLink::SameLevel { link }),
        "a same-storey link must be rejected",
    );
}

#[test]
fn empty_situation_builds_empty_graph() {
    let Some(graph) = ok_graph(&Situation::new()) else {
        return;
    };
    assert!(graph.is_empty());
    assert_eq!(graph.len(), 0);
    assert_eq!(graph.links_from(&key(0, 0, 0)).count(), 0);
}

#[test]
fn link_kind_one_way_predicate() {
    assert!(!*LinkKind::stair().is_one_way());
    assert!(!*LinkKind::ladder().is_one_way());
    assert!(
        *LinkKind::Stair {
            one_way: OneWay::forward_only(),
        }
        .is_one_way()
    );
    assert!(
        *LinkKind::Ladder {
            one_way: OneWay::forward_only(),
        }
        .is_one_way()
    );
    assert!(
        !*LinkKind::Ladder {
            one_way: OneWay::bidirectional(),
        }
        .is_one_way()
    );
}

#[test]
fn traversable_link_yields_link_tu_priced_hop() {
    let lower = key(5, 5, 0);
    let upper = key(5, 5, 1);
    let link = VerticalLink::new(lower, upper, LinkKind::stair());
    let situation = situation_with(&[lower, upper], vec![link]);

    let Some(graph) = ok_graph(&situation) else {
        return;
    };

    let link_tu = LinkTu::new(7);
    let hops: Vec<_> = traversable_links(lower, &graph, link_tu).collect();

    assert_eq!(
        hops,
        vec![(upper, Tu::new(7))],
        "the hop reaches the `to` endpoint at the LinkTu cost"
    );
    let (_, cost) = hops[0];
    assert_eq!(*cost, *link_tu, "the hop cost equals the LinkTu passed in");
}

#[test]
fn cost_is_flat_link_tu_regardless_of_kind() {
    let lower = key(3, 3, 0);
    let upper = key(3, 3, 1);
    let link_tu = LinkTu::new(9);

    for kind in [LinkKind::stair(), LinkKind::ladder()] {
        let link = VerticalLink::new(lower, upper, kind);
        let situation = situation_with(&[lower, upper], vec![link]);
        let Some(graph) = ok_graph(&situation) else {
            return;
        };
        let hops: Vec<_> = traversable_links(lower, &graph, link_tu).collect();
        assert_eq!(
            hops,
            vec![(upper, Tu::new(9))],
            "every link kind prices a hop at the same single LinkTu",
        );
    }
}

#[test]
fn one_way_link_is_not_traversable_from_to() {
    let lower = key(2, 3, 0);
    let upper = key(2, 3, 1);
    let link = VerticalLink::new(
        lower,
        upper,
        LinkKind::Stair {
            one_way: OneWay::forward_only(),
        },
    );
    let situation = situation_with(&[lower, upper], vec![link]);

    let Some(graph) = ok_graph(&situation) else {
        return;
    };
    let link_tu = LinkTu::new(5);

    assert_eq!(
        traversable_links(lower, &graph, link_tu).collect::<Vec<_>>(),
        vec![(upper, Tu::new(5))],
        "a one-way link is traversable from its `from` endpoint",
    );
    assert_eq!(
        traversable_links(upper, &graph, link_tu).count(),
        0,
        "a one-way link is NOT traversable from its `to` endpoint",
    );
}

#[test]
fn bidirectional_link_is_traversable_both_ways() {
    let lower = key(6, 6, 0);
    let upper = key(6, 6, 1);
    let link = VerticalLink::new(lower, upper, LinkKind::ladder());
    let situation = situation_with(&[lower, upper], vec![link]);

    let Some(graph) = ok_graph(&situation) else {
        return;
    };
    let link_tu = LinkTu::new(3);

    assert_eq!(
        traversable_links(lower, &graph, link_tu).collect::<Vec<_>>(),
        vec![(upper, Tu::new(3))],
        "a bidirectional link is traversable forward",
    );
    assert_eq!(
        traversable_links(upper, &graph, link_tu).collect::<Vec<_>>(),
        vec![(lower, Tu::new(3))],
        "a bidirectional link is traversable in reverse too",
    );
}

#[test]
fn no_link_yields_no_hops() {
    let Some(graph) = ok_graph(&Situation::new()) else {
        return;
    };
    assert_eq!(
        traversable_links(key(0, 0, 0), &graph, LinkTu::new(4)).count(),
        0,
        "an origin with no departing link yields nothing",
    );
}
