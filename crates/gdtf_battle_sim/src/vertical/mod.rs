//! Vertical-link graph: the model's authored **stair / ladder** links between
//! storeys — the *only* way a ganger changes storey
//! (`docs/combat/combat.md`: "gangers change storeys **only over authored
//! stair/ladder links** (a situation's `vertical_links`, validated and poured
//! into the movement graph)"). The authored situation carries these stair/ladder
//! vertical links — every placement an optional storey, position always the pair
//! (cell, level) — as documented on the [`crate::situation`] module (the
//! setup-on-entry source of truth).
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
//!    `0..`[`MAX_LEVELS`](crate::metric::MAX_LEVELS); neither endpoint cell is
//!    DANGLING (it must appear among the situation's authored cells — walls,
//!    scatter, or slabs, the cell-existence source); and the two endpoints are on
//!    DIFFERENT storeys.
//! 3. The lookup index — a [`VerticalLinkGraph`] Bevy
//!    [`Resource`](bevy::prelude::Resource) built from the VALIDATED links,
//!    answering [`links_from`](VerticalLinkGraph::links_from): the links departing a
//!    given `(cell, level)`. A link is indexed in BOTH directions unless its kind is
//!    [`one-way`](LinkKind::is_one_way).
//! 4. The **traversal gate** (E7 · GTW-12c) — [`traversable_links`], a PURE read
//!    over the index that yields each reachable cross-storey neighbour and its
//!    flat [`LinkTu`](crate::tuning::LinkTu) hop cost (`docs/combat/visibility.md`
//!    §48: one `link_tu` per hop regardless of stair / ladder kind). One-way
//!    directionality is honoured by the index alone (a one-way link is recorded
//!    under its [`from`](VerticalLink::from) endpoint only), not re-checked.
//!
//! Scope: this is **graph + validation + the traversal gate**. There is no
//! pathfinding / route assembly here — that is the route core (GTW-352).
//! [`links_from`](VerticalLinkGraph::links_from) is the existence query (what
//! links leave here); [`traversable_links`] adds the per-hop cost; neither is a path.

mod graph;
mod links;
mod traverse;

#[cfg(test)]
mod test;

pub use graph::{InvalidVerticalLink, VerticalLinkGraph, build_vertical_link_graph};
pub use links::{LinkKind, OneWay, VerticalLink};
pub use traverse::traversable_links;
