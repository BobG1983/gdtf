//! The DEV-ONLY procgen-visualizer model (GTW-434) — the ordered placement sequence the
//! STEP / AUTO controls reveal, plus the domain newtypes the visualizer reasons in.
//!
//! The visualizer is a pure VIEW onto the sim's space-packing placement (the one-way
//! sim→presenter boundary): it RUNS the sim's
//! [`assemble_placement`](gdtf_battle_sim::procgen::assemble_placement) and
//! [`fill_placement`](gdtf_battle_sim::procgen::fill_placement) against the loaded UUID-keyed
//! [`PrefabRegistry`](gdtf_battle_sim::PrefabRegistry) for a theme + grid-size + seed, then
//! projects the resulting [`FilledPlacement`](gdtf_battle_sim::FilledPlacement) into an
//! ORDERED list of [`VizQuad`]s — player, enemy, then every fill prefab in placement order.
//! It never mutates combat/sim state; it only reads the placement.
//!
//! It is inserted as a [`Resource`](bevy::prelude::Resource) `OnEnter(DebugProcgenVisualizer)`
//! (built from the live registry / situation, or EMPTY when none is present) and removed
//! `OnExit`, per the project's state-scoped-resource convention (`bevy-traps.md` #1) — so
//! every system reading it guards with `run_if(resource_exists::<ProcgenViz>)` /
//! `Option<Res<…>>`.
//!
//! The whole module is `#[cfg(debug_assertions)]`-gated by its parent (`procgen_viz`), so it
//! compiles out of release (C4).

mod quad;
mod reveal;
mod units;
mod viz;

// The test_support ledger in `procgen_viz/mod.rs` publicly re-exports `model::ProcgenViz`
// and `model::QuadTint`, so these two re-exports must widen in lockstep with their
// `support_item!` definitions (a capped `pub(in ...)` re-export would hit E0365 under
// `test-support`).
crate::support_use!(quad::QuadTint;);
crate::support_use!(viz::ProcgenViz;);

// The remaining out-of-model consumer surface (components, systems, config) — narrower
// re-exports of `support_item!`-widened items are legal in this direction.
pub(in crate::states::running::procgen_viz) use quad::VizQuad;
pub(in crate::states::running::procgen_viz) use reveal::RevealIndex;
pub(in crate::states::running::procgen_viz) use units::{BoardExtent, QuadRect};
pub(in crate::states::running::procgen_viz) use viz::default_viz_seed;
