//! Vertical-link **traversal gate** (E7 · GTW-12c): the pure
//! [`traversable_links`] read over the validated
//! [`VerticalLinkGraph`](crate::vertical::VerticalLinkGraph) — the cross-storey
//! neighbours a ganger can reach from a `(cell, level)`, each priced at the flat
//! [`LinkTu`] hop cost.
//!
//! This is a PURE function over the existing graph index — no new traversal
//! state, no new resource, no `&mut World`. It composes
//! [`links_from`](crate::vertical::VerticalLinkGraph::links_from) (which already
//! enforces directionality: a one-way link is indexed under its
//! [`from`](crate::vertical::VerticalLink::from) endpoint ONLY, so
//! `links_from(to)` simply never yields it) with the single
//! [`LinkTu`]→[`Tu`] cost conversion.
//!
//! Scope: the traversal GATE + cost only — what cross-storey neighbours leave a
//! cell and at what TU price. Route assembly (pathfinding over these hops) is
//! GTW-352.

use crate::{ganger::Tu, metric::CellLevel, tuning::LinkTu, vertical::VerticalLinkGraph};

/// The cross-storey neighbours reachable from `origin` over a vertical link, each
/// paired with its flat [`Tu`] hop cost (`docs/combat/visibility.md` §48: a
/// crossing prices the single `link_tu`, regardless of stair / ladder kind).
///
/// A PURE read built ON
/// [`VerticalLinkGraph::links_from`](crate::vertical::VerticalLinkGraph::links_from):
/// for each link departing `origin` it yields the OTHER endpoint of that link —
/// the reachable cross-storey neighbour — and the FLAT `link_tu` converted to a
/// [`Tu`]. For a link departed from its [`from`](crate::vertical::VerticalLink::from)
/// endpoint the neighbour is [`to`](crate::vertical::VerticalLink::to); for a
/// bidirectional link departed from its `to` endpoint (the reverse hop) the
/// neighbour is `from`. The cost is the SAME single `link_tu` for every link kind
/// — there is no per-kind (stair / ladder) cost (per §48; the per-kind variant
/// would need a docs change first).
///
/// **One-way directionality is honoured by the index, not re-checked here.** The
/// graph records a [`one-way`](crate::vertical::LinkKind::is_one_way) link under
/// its [`from`](crate::vertical::VerticalLink::from) endpoint ONLY, so
/// `links_from(to)` never yields it and this function therefore never offers the
/// reverse hop. A bidirectional link is indexed under BOTH endpoints, so it is
/// traversable both ways.
///
/// An `origin` with no departing link yields an empty iterator (never a panic).
/// This is the traversal GATE + cost only — it does not assemble a route (GTW-352).
pub fn traversable_links(
    origin: CellLevel,
    graph: &VerticalLinkGraph,
    link_tu: LinkTu,
) -> impl Iterator<Item = (CellLevel, Tu)> {
    // The single LinkTu -> Tu cost conversion: LinkTu derefs to its inner `u8`
    // magnitude, which `Tu::new` wraps — mirroring the MoveCost -> Tu lookup-site
    // conversion (`Tu::new(*cost)`). ONE flat cost for every link kind (§48).
    let cost = Tu::new(*link_tu);
    // `links_from` borrows its `&origin` for the lifetime of the iterator it
    // returns, so the owned `origin` must outlive that iterator. Carry it inside a
    // `once` adapter (a single yield that OWNS `origin`); the inner `links_from`
    // then borrows the adapter's owned value, so the whole chain is lazy at the
    // outer level. The reachable neighbour is the OTHER endpoint relative to
    // `origin`: `to` for a forward hop, `from` for a bidirectional reverse hop.
    // (The graph index — not this read — enforces one-way directionality: a
    // one-way link is never recorded under its `to`, so `links_from(to)` never
    // yields it.)
    std::iter::once(origin)
        .flat_map(move |origin| {
            graph
                .links_from(&origin)
                .map(|link| {
                    if link.from == origin {
                        link.to
                    } else {
                        link.from
                    }
                })
                .collect::<Vec<_>>()
        })
        .map(move |to| (to, cost))
}
