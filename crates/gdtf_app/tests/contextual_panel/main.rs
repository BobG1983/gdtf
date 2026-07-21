//! GTW-294 — the battlescape CONTEXTUAL PANEL (bottom-right), driven through the REAL app stack.
//!
//! These headless `GdtfTestAppBuilder` integration tests drive the genuine state machine down to
//! `BattleScapeState::BattleRunning`, where the real contextual-panel plugin spawns its tree on
//! `OnEnter`, tears it down on `OnExit`, and runs its live detection + press-routing systems.
//! They cover the contract:
//!
//! - **Scaffold** — on entering `BattleRunning` the `ContextualPanelRoot` box and one button
//!   per registered contextual act exist; the box + buttons spawn `Visibility::Hidden`;
//!   the box is a CHILD of the bottom bar so it is laid out inside the bottom panel and draws on
//!   top of the bar's fill with no `GlobalZIndex` of its own (GTW-726); on exiting `BattleRunning`
//!   the whole subtree (box → buttons) is despawned (battle-scoped lifecycle).
//! - **Layout geometry** (GTW-726, real `bevy_ui` layout pass) — with an act offered, the panel
//!   root's REAL computed rect is contained INSIDE the bottom bar's rect (never overflowing up
//!   onto the map), proving the detach fix; a parent-child + no-respawn assertion pins the
//!   mutate-in-place update path.
//! - **Detection** — a selected actor with an 8-adjacent downed ENEMY offers Execute (button +
//!   panel visible, target set); with an 8-adjacent unstabilized downed ALLY offers Stabilize;
//!   with no adjacent downed neighbour the panel + all buttons hide and both targets clear.
//! - **Reactive, no respawn** — moving the actor away (or clearing selection) hides the panel
//!   while the SAME button entities persist (a `Visibility` toggle, never a despawn/respawn).
//! - **Press → intent** — with an Execute target offered, pressing the Execute button drives
//!   the REAL GTW-571 seam (button → the act's generic press router → the act's buffered
//!   `PendingContextualIntents` queue → the act's generic drain) to emit one
//!   `ExecuteDownedRequested` for the selection as actor over the carried downed target,
//!   the SAME update (the Q5 same-frame guarantee).

mod actors;
mod door;
mod downed_acts;
mod emplacement;
mod harness;
mod layout_geometry;
mod melee;
mod real_layout_harness;
mod scaffold;
mod shove;
mod throw;
