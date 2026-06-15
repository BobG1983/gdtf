//! Vertical-link graph: the model's authored **stair / ladder** links between
//! storeys — the *only* way a ganger changes storey
//! (`docs/combat/combat.md`: "gangers change storeys **only over authored
//! stair/ladder links** (a situation's `vertical_links`, validated and poured
//! into the movement graph)"; `docs/architecture.md`: the situation carries
//! "stair/ladder vertical links — every placement an optional storey, position
//! always the pair (cell, level)").
//!
//! This is the E1.10 vertical-link slice. It supplies three things:
//!
//! 1. The authored shape — a [`VerticalLink`] (two `(cell, level)` endpoints +
//!    a [`LinkKind`]). The authored list lives on the canonical
//!    [`Situation::vertical_links`](crate::situation::Situation::vertical_links)
//!    (GTW-158 moved it there from the GTW-156 placeholder).
//! 2. **Setup-time validation** — [`build_vertical_link_graph`] checks every
//!    authored link against three rules and returns a TYPED
//!    [`InvalidVerticalLink`] error (NEVER a panic): each endpoint's level is in
//!    `0..`[`MAX_LEVELS`]; neither endpoint cell is DANGLING (it must appear among
//!    the situation's authored cells — walls, scatter, or slabs, the
//!    cell-existence source); and the two endpoints are on DIFFERENT storeys.
//! 3. The lookup index — a [`VerticalLinkGraph`] Bevy [`Resource`] built from the
//!    VALIDATED links, answering [`links_from`](VerticalLinkGraph::links_from):
//!    the links departing a given `(cell, level)`. A link is indexed in BOTH
//!    directions unless its kind is [`one-way`](LinkKind::is_one_way).
//!
//! Scope: this is **graph + validation ONLY**. There is no traversal,
//! pathfinding, or movement cost here — that is the multi-level movement work
//! (GTW-12). [`links_from`](VerticalLinkGraph::links_from) is the existence query
//! (what links leave here), not a path.

use bevy::{
    platform::collections::{HashMap, HashSet},
    prelude::{Deref, Resource},
};
use serde::Deserialize;

use crate::{
    metric::{CellLevel, MAX_LEVELS},
    situation::Situation,
};

/// Whether a [`VerticalLink`]'s kind is **one-way** — traversable only in the
/// authored `(from → to)` direction.
///
/// A named newtype over `bool` (no-bare-types: a link's directionality is a
/// load-bearing domain value — it gates whether [`VerticalLinkGraph`] indexes the
/// reverse direction — not a bare boolean). `true` means one-way (forward only);
/// `false` (the default) means bidirectional (climbed both up and down). Private
/// inner + derived [`Deref`] (house style, matching `Aiming`/`Destroyed`).
/// `#[serde(transparent)]` lets an authored directionality parse as a bare boolean.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Deserialize)]
#[serde(transparent)]
pub struct OneWay(bool);

impl OneWay {
    /// Build a directionality flag from its boolean state — `true` for a one-way
    /// link, `false` for a bidirectional one.
    #[must_use]
    pub const fn new(one_way: bool) -> Self {
        Self(one_way)
    }

    /// A bidirectional link — traversable in both directions (the default).
    #[must_use]
    pub const fn bidirectional() -> Self {
        Self(false)
    }

    /// A one-way link — traversable only in the authored `(from → to)` direction.
    #[must_use]
    pub const fn forward_only() -> Self {
        Self(true)
    }

    /// Whether this flag denotes a one-way link.
    #[must_use]
    pub const fn is_one_way(self) -> bool {
        self.0
    }
}

/// What kind of authored vertical connection a [`VerticalLink`] is, and whether
/// it may be traversed in both directions.
///
/// A named domain enum (no-bare-types: a link's kind is a domain value, not a
/// bare bool/`u8`). The two physical kinds are the ones `docs/combat/combat.md`
/// names — **stair** and **ladder** — each carrying a [`OneWay`] flag. By default
/// a link is **bidirectional** (a stair/ladder is climbed both up and down); a
/// one-way link (e.g. a drop a ganger can descend but not climb back) is
/// traversable only from its lower-listed endpoint to its higher-listed one (the
/// authored `(from → to)` direction). Derives [`Deserialize`] so an authored
/// situation names its links' kinds (`Stair`/`Ladder`) and one-way flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
pub enum LinkKind {
    /// A staircase between storeys.
    Stair {
        /// Whether the stair is traversable only in the authored `(from → to)`
        /// direction ([`OneWay::forward_only`]) or bidirectional ([`OneWay::bidirectional`], the default).
        one_way: OneWay,
    },
    /// A ladder between storeys.
    Ladder {
        /// Whether the ladder is traversable only in the authored `(from → to)`
        /// direction ([`OneWay::forward_only`]) or bidirectional ([`OneWay::bidirectional`], the default).
        one_way: OneWay,
    },
}

impl LinkKind {
    /// A normal, bidirectional staircase.
    #[must_use]
    pub const fn stair() -> Self {
        Self::Stair {
            one_way: OneWay::bidirectional(),
        }
    }

    /// A normal, bidirectional ladder.
    #[must_use]
    pub const fn ladder() -> Self {
        Self::Ladder {
            one_way: OneWay::bidirectional(),
        }
    }

    /// Whether this kind is one-way — traversable only in the authored
    /// `(from → to)` direction. `false` means bidirectional (climbed both ways).
    #[must_use]
    pub const fn is_one_way(self) -> bool {
        match self {
            Self::Stair { one_way } | Self::Ladder { one_way } => one_way.is_one_way(),
        }
    }
}

/// One authored vertical connection between two storeys — its two `(cell, level)`
/// endpoints and its [`LinkKind`].
///
/// A named struct rather than a bare `(CellLevel, CellLevel, LinkKind)` tuple so
/// the authored shape is self-describing. The endpoints are the GTW-151
/// [`CellLevel`] pair: [`from`](VerticalLink::from) is the authored departure
/// `(cell, level)`, [`to`](VerticalLink::to) the authored arrival. For a
/// bidirectional kind the link is traversable both ways; for a
/// [`one-way`](LinkKind::is_one_way) kind only `from → to`. Derives
/// [`Deserialize`] so an authored situation names each link's two endpoints + kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
pub struct VerticalLink {
    /// The authored departure endpoint — `(cell, level_from)`.
    pub from: CellLevel,
    /// The authored arrival endpoint — `(cell, level_to)`.
    pub to:   CellLevel,
    /// The kind of connection (stair / ladder, with its [`OneWay`] flag).
    pub kind: LinkKind,
}

impl VerticalLink {
    /// Build a vertical link from its two `(cell, level)` endpoints and its
    /// [`LinkKind`].
    #[must_use]
    pub const fn new(from: CellLevel, to: CellLevel, kind: LinkKind) -> Self {
        Self { from, to, kind }
    }
}

/// A typed reason an authored [`VerticalLink`] FAILED setup-time validation —
/// returned by [`build_vertical_link_graph`], **never panicked** (the no-panic
/// workspace lints make this binding).
///
/// One variant per validation rule (no-bare-types: a validation failure is a
/// domain value, not a bare `String`/`bool`). Each carries the offending
/// [`VerticalLink`] so the caller (the GTW-158 setup orchestration) can report
/// exactly which authored link was rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvalidVerticalLink {
    /// An endpoint's storey index is outside `0..`[`MAX_LEVELS`] — the link
    /// reaches a storey that does not exist.
    LevelOutOfRange {
        /// The rejected link.
        link: VerticalLink,
    },
    /// An endpoint cell does NOT appear among the situation's authored cells
    /// (walls, scatter, or slabs) — the link dangles off a `(cell, level)` that no
    /// authored tile occupies.
    DanglingCell {
        /// The rejected link.
        link: VerticalLink,
    },
    /// Both endpoints are on the SAME storey — a vertical link must change
    /// storey (`from.z != to.z`).
    SameLevel {
        /// The rejected link.
        link: VerticalLink,
    },
}

/// The validated vertical-link graph — the model's index of authored
/// stair / ladder links, keyed by departure `(cell, level)` for fast
/// [`links_from`](VerticalLinkGraph::links_from) lookup.
///
/// A Bevy [`Resource`] (one graph per battle), built ONLY from links that passed
/// [`build_vertical_link_graph`]'s validation. The index is **bidirectional by
/// construction**: a normal link is reachable from BOTH its endpoints, so
/// `links_from(from)` and `links_from(to)` each yield it; a
/// [`one-way`](LinkKind::is_one_way) link is indexed from its
/// [`from`](VerticalLink::from) endpoint only.
///
/// Scope is the existence query alone — there is no traversal / pathfinding /
/// movement cost here (that is GTW-12).
#[derive(Resource, Debug, Clone, Default)]
pub struct VerticalLinkGraph {
    /// The validated links, owned once so the per-key index can borrow them.
    links:     Vec<VerticalLink>,
    /// Departure `(cell, level)` → the indices into [`links`](Self::links) of the
    /// links departing there. A normal link is recorded under BOTH endpoints; a
    /// one-way link under its [`from`](VerticalLink::from) only.
    by_origin: HashMap<CellLevel, Vec<usize>>,
}

impl VerticalLinkGraph {
    /// The links departing `origin` (a `(cell, level)`) — the existence query.
    ///
    /// Yields every validated [`VerticalLink`] one can leave `origin` over: a
    /// bidirectional link from either of its endpoints, a
    /// [`one-way`](LinkKind::is_one_way) link from its
    /// [`from`](VerticalLink::from) endpoint only. An `origin` with no departing
    /// link yields an empty iterator (never a panic). This is existence ONLY — it
    /// does NOT compute reachability, cost, or a path (GTW-12).
    pub fn links_from(&self, origin: &CellLevel) -> impl Iterator<Item = &VerticalLink> {
        self.by_origin
            .get(origin)
            .into_iter()
            .flatten()
            .filter_map(|&i| self.links.get(i))
    }

    /// The total number of validated links in the graph (each authored link
    /// counted once, regardless of how many directions it is indexed under).
    #[must_use]
    pub const fn len(&self) -> usize {
        self.links.len()
    }

    /// Whether the graph holds no links.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.links.is_empty()
    }
}

/// Validate every authored [`VerticalLink`] in `situation`, then build the
/// [`VerticalLinkGraph`] index from the validated links — the setup-time
/// constructor.
///
/// Validation runs per link, in author order, and returns the FIRST failure as a
/// typed [`InvalidVerticalLink`] (NEVER a panic). The three rules (C3):
///
/// 1. **Level in range** — both `from.z` and `to.z` are in `0..`[`MAX_LEVELS`];
///    out of range → [`InvalidVerticalLink::LevelOutOfRange`].
/// 2. **No dangling endpoint** — each endpoint's `(cell, level)` must appear among
///    the situation's authored cells ([`Situation::authored_cells`](crate::situation::Situation::authored_cells):
///    walls, scatter, or slabs — the cell-existence source); a missing endpoint →
///    [`InvalidVerticalLink::DanglingCell`].
/// 3. **Different storeys** — `from.z != to.z`; a same-level link →
///    [`InvalidVerticalLink::SameLevel`].
///
/// On success the index records each VALID link under its departure endpoint(s):
/// both endpoints for a bidirectional kind, the [`from`](VerticalLink::from)
/// endpoint only for a [`one-way`](LinkKind::is_one_way) kind (C4).
///
/// # Errors
///
/// Returns [`InvalidVerticalLink`] for the first authored link that fails any of
/// the three rules above.
pub fn build_vertical_link_graph(
    situation: &Situation,
) -> Result<VerticalLinkGraph, InvalidVerticalLink> {
    // The cell-existence source: every (cell, level) the situation authors via a
    // wall, scatter prop, or slab, used for the dangling check.
    let authored: HashSet<CellLevel> = situation.authored_cells().collect();

    let mut graph = VerticalLinkGraph::default();

    for &link in &situation.vertical_links {
        validate_link(&link, &authored)?;

        let index = graph.links.len();
        graph.links.push(link);
        graph.by_origin.entry(link.from).or_default().push(index);
        if !link.kind.is_one_way() {
            graph.by_origin.entry(link.to).or_default().push(index);
        }
    }

    Ok(graph)
}

/// Validate one [`VerticalLink`] against the three setup rules, given the set of
/// `(cell, level)` the situation's terrain occupies. Returns the typed failure or
/// `Ok(())`.
fn validate_link(
    link: &VerticalLink,
    authored: &HashSet<CellLevel>,
) -> Result<(), InvalidVerticalLink> {
    let max_level = i32::from(MAX_LEVELS);
    let in_range = |key: &CellLevel| key.z >= 0 && key.z < max_level;

    if !in_range(&link.from) || !in_range(&link.to) {
        return Err(InvalidVerticalLink::LevelOutOfRange { link: *link });
    }
    if !authored.contains(&link.from) || !authored.contains(&link.to) {
        return Err(InvalidVerticalLink::DanglingCell { link: *link });
    }
    if link.from.z == link.to.z {
        return Err(InvalidVerticalLink::SameLevel { link: *link });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        metric::{Cell, Level},
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
}
