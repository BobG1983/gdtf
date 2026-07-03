//! One module per **struck kind** — each owns its whole fold (input resolution →
//! model mutation → verdict construction) and its per-kind verdict payload type
//! (GTW-573 C3 / P10).
//!
//! Adding a new struck kind is: one sibling module here, one
//! [`HitVerdict`](super::report::HitVerdict) variant, and one delegation arm in
//! [`resolve_and_apply`](super::resolve_and_apply) — the exhaustive verdict matches
//! (the fire bridge, the presenter classifier) then FAIL TO COMPILE until the new
//! kind is bridged, which is the point.
//!
//! Every fold here is statically dispatched (a plain function per kind — no trait
//! object, no deferred commands-ext: the severity-gated draw discipline is
//! load-bearing and SYNCHRONOUS) and takes NO ECS query — the GTW-323
//! query-disjointness stays at the call boundary (`fire`'s composer resolves the
//! borrowed views and hands them in).

pub(super) mod cover;
pub(super) mod ganger;
pub(super) mod ground;
pub(super) mod slab;
