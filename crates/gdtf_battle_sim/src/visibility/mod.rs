//! The squad fog-of-war **three-state visibility model** — VISIBLE / EXPLORED /
//! UNSEEN per `(cell, level)`, keyed [`CellLevel`](crate::metric::CellLevel) — its
//! resource, its **pure read seams**, and the **pure union / accrual helpers**
//! (GTW-340, leaf 4 of the GTW-13 FOV epic; `docs/combat/visibility.md`).
//!
//! Computed model-side from the **squad-combined** point of view: one fog for the
//! player side, the union of every conscious player-faction ganger's FOV — not
//! per-ganger overlays. The presenter (GTW-342) presents it; it never owns it (the
//! model/view split, ADR-0001).
//!
//! This module provides the **data + helpers** (GTW-340) and the **writer system +
//! trigger gate** that wire them onto the battle (GTW-341):
//!
//! - [`SquadVisibility`] — the Bevy [`Resource`](bevy::prelude::Resource) holding the
//!   VISIBLE and EXPLORED [`HashSet`](bevy::platform::collections::HashSet)`<`[`CellLevel`](crate::metric::CellLevel)`>`
//!   (UNSEEN is the implicit complement of EXPLORED; EXPLORED is **monotone**).
//! - The pure **read seams** the consumers (GTW-11 fog gate, GTW-70 AI, GTW-38
//!   reaction fire) call — all set lookups, no geometry / recompute / grid input:
//!   [`SquadVisibility::is_cell_visible`], [`SquadVisibility::is_cell_explored`],
//!   [`SquadVisibility::visible_cells`], and [`is_ganger_visible`] (gated by an
//!   explicit [`FactionRelation`]: own squad trivially visible, an enemy visible iff
//!   its cell is squad-VISIBLE).
//! - The pure **value transforms** GTW-341's recompute system calls:
//!   [`union_fov`] (the squad VISIBLE union over a disc-bounded authored/occupied
//!   candidate set, resolving each candidate's band the shot-pipeline way) and
//!   [`accrue`] (VISIBLE replaces, EXPLORED grows monotonically).
//!
//! - The **writer system** [`recompute_visibility`] (GTW-341, leaf 5) — the SOLE mutator
//!   of [`SquadVisibility`]: it gathers the conscious player-faction observers and runs
//!   [`union_fov`] then [`accrue`] on every trigger (a ganger move / re-pose / life flip,
//!   a [`CoverDestroyed`](crate::occupancy_sync::CoverDestroyed) message, or a
//!   [`BattleReady`](crate::battle::BattleReady) spawn-FOV), and its trigger gate
//!   [`should_recompute_visibility`]. Wired by
//!   [`BattleSimPlugin`](crate::battle::BattleSimPlugin) after the `occupancy_sync` grid
//!   maintenance, inside the [`BattleInProgress`](crate::battle::BattleInProgress)-gated
//!   `Simulate` band.
//!
//! The GTW-340 helpers are pure (render-free, deterministic, RNG-free, no `&mut World`);
//! the GTW-341 writer is param-only (`Query` / `Res` / `ResMut`), no `&mut World`.

mod compute;
mod recompute;
mod squad;

#[cfg(test)]
mod test;

pub use compute::{FovObserver, accrue, union_fov};
pub use recompute::{recompute_visibility, should_recompute_visibility};
pub use squad::{FactionRelation, SquadVisibility, is_ganger_visible};
