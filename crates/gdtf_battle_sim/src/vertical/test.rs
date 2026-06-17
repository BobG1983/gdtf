use super::{
    InvalidVerticalLink, LinkKind, OneWay, VerticalLink, VerticalLinkGraph,
    build_vertical_link_graph,
};
use crate::{
    metric::{Cell, CellLevel, Level, MAX_LEVELS},
    situation::Situation,
};

/// Build a `(cell, level)` key from raw coordinates.
fn key(x: i32, y: i32, level: u8) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(level))
}

/// A situation whose **slabs** occupy every `(cell, level)` in `cells` — a
/// cell-existence source for the dangling check — plus the given links. Slabs
/// are the lightest authored-cell carrier (just a `CellLevel`), so they isolate
/// the vertical-link rules under test from wall / scatter authoring.
fn situation_with(cells: &[CellLevel], links: Vec<VerticalLink>) -> Situation {
    Situation {
        slabs: cells.to_vec(),
        vertical_links: links,
        ..Situation::new()
    }
}

/// Build the graph, asserting it is `Ok`, and return it — or assert-fail and
/// return `None`. Keeps the Ok-needing tests free of `unwrap`/`expect`/`panic`
/// (all denied in tests too).
fn ok_graph(situation: &Situation) -> Option<VerticalLinkGraph> {
    let result = build_vertical_link_graph(situation);
    assert!(result.is_ok(), "expected a valid graph, got {result:?}");
    result.ok()
}

/// C7(a) — valid links build successfully and `links_from` returns the correct
/// set, including the REVERSE direction for a bidirectional link.
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

    // Forward direction: departing the lower endpoint yields the link.
    let from_lower: Vec<_> = graph.links_from(&lower).collect();
    assert_eq!(
        from_lower,
        vec![&link],
        "the link departs the lower endpoint"
    );

    // Reverse direction: a bidirectional link is ALSO reachable from `to`.
    let from_upper: Vec<_> = graph.links_from(&upper).collect();
    assert_eq!(
        from_upper,
        vec![&link],
        "a bidirectional link departs the upper endpoint too (reverse direction)",
    );

    // A cell with no link departing it yields nothing.
    assert_eq!(graph.links_from(&key(9, 9, 0)).count(), 0);
}

/// A ONE-WAY link is indexed from its `from` endpoint ONLY — the reverse
/// direction is not traversable.
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

/// C7(b) — a link with `level_from = MAX_LEVELS` returns
/// `Err(LevelOutOfRange)`, NOT a panic. (`MAX_LEVELS` is the first
/// out-of-range storey: valid levels are `0..MAX_LEVELS`.)
#[test]
fn level_at_max_levels_is_out_of_range() {
    let bad = key(1, 1, MAX_LEVELS);
    let ok = key(1, 1, 0);
    // Author the cells so the dangling check would pass — isolating the
    // level-range rule.
    let link = VerticalLink::new(bad, ok, LinkKind::ladder());
    let situation = situation_with(&[bad, ok], vec![link]);

    // Compare the error directly (the Ok variant `VerticalLinkGraph` is not
    // `PartialEq`, so compare `.err()` — an `Option<InvalidVerticalLink>`).
    let result = build_vertical_link_graph(&situation);
    assert_eq!(
        result.err(),
        Some(InvalidVerticalLink::LevelOutOfRange { link }),
        "level_from = MAX_LEVELS must be rejected as out of range",
    );
}

/// C7(c) — a dangling-cell link (an endpoint cell NOT among the situation's
/// authored cells) returns `Err(DanglingCell)`.
#[test]
fn dangling_endpoint_cell_is_rejected() {
    let present = key(4, 4, 0);
    let missing = key(4, 4, 1); // never authored
    let link = VerticalLink::new(present, missing, LinkKind::stair());
    // Only `present` is authored — `missing` dangles.
    let situation = situation_with(&[present], vec![link]);

    let result = build_vertical_link_graph(&situation);
    assert_eq!(
        result.err(),
        Some(InvalidVerticalLink::DanglingCell { link }),
        "an endpoint cell absent from authored cells must be rejected as dangling",
    );
}

/// C7(d) — `level_from == level_to` returns `Err(SameLevel)`.
#[test]
fn same_level_link_is_rejected() {
    let a = key(7, 7, 2);
    let b = key(8, 8, 2); // same storey, different cell
    let link = VerticalLink::new(a, b, LinkKind::stair());
    let situation = situation_with(&[a, b], vec![link]);

    let result = build_vertical_link_graph(&situation);
    assert_eq!(
        result.err(),
        Some(InvalidVerticalLink::SameLevel { link }),
        "a same-storey link must be rejected",
    );
}

/// An empty situation builds an empty graph (no links, every lookup empty) —
/// the trivial valid case, and the default for a situation with no authored
/// vertical links.
#[test]
fn empty_situation_builds_empty_graph() {
    let Some(graph) = ok_graph(&Situation::new()) else {
        return;
    };
    assert!(graph.is_empty());
    assert_eq!(graph.len(), 0);
    assert_eq!(graph.links_from(&key(0, 0, 0)).count(), 0);
}

/// `LinkKind` constructors and the one-way predicate behave as documented:
/// the convenience constructors are bidirectional, the explicit flag is read
/// back faithfully.
#[test]
fn link_kind_one_way_predicate() {
    assert!(!LinkKind::stair().is_one_way());
    assert!(!LinkKind::ladder().is_one_way());
    assert!(
        LinkKind::Stair {
            one_way: OneWay::forward_only(),
        }
        .is_one_way()
    );
    assert!(
        LinkKind::Ladder {
            one_way: OneWay::forward_only(),
        }
        .is_one_way()
    );
    assert!(
        !LinkKind::Ladder {
            one_way: OneWay::bidirectional(),
        }
        .is_one_way()
    );
}
