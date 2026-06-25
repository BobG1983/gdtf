//! Setup-time validation + the lookup index — the typed [`InvalidVerticalLink`]
//! rejection, the [`VerticalLinkGraph`] resource, and
//! [`build_vertical_link_graph`] (with its per-link [`validate_link`]).

use bevy::{
    platform::collections::{HashMap, HashSet},
    prelude::Resource,
};

use crate::{
    metric::{CellLevel, MAX_LEVELS},
    situation::Situation,
    vertical::links::VerticalLink,
};

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
/// [`one-way`](crate::vertical::LinkKind::is_one_way) link is indexed from its
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
    /// [`one-way`](crate::vertical::LinkKind::is_one_way) link from its
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

    /// Every validated link in the graph, in author order (each authored link
    /// yielded once, regardless of how many directions it is indexed under).
    ///
    /// The whole-graph read seam — distinct from
    /// [`links_from`](VerticalLinkGraph::links_from), which yields only the links
    /// departing a given `origin`. The GTW-359 presenter draw iterates this to render
    /// one stair / ladder tile per link endpoint on the active storey (it needs every
    /// link's endpoints + kind, not the per-origin departure set). Existence only — no
    /// reachability / cost / path (GTW-12).
    pub fn links(&self) -> impl Iterator<Item = &VerticalLink> {
        self.links.iter()
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
/// endpoint only for a [`one-way`](crate::vertical::LinkKind::is_one_way) kind (C4).
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
