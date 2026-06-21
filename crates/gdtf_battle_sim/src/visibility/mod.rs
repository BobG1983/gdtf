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
//! This leaf provides the **data + helpers**, NOT the system trigger wiring:
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
//! The recompute SYSTEM that wires these onto every `Changed<Position>` move step (and
//! the other triggers) is GTW-341 (leaf 5) — NOT here. Everything in this module is
//! pure: render-free, deterministic, RNG-free, no `&mut World` / `Commands`.

mod compute;
mod squad;

#[cfg(test)]
mod test;

pub use compute::{FovObserver, accrue, union_fov};
pub use squad::{FactionRelation, SquadVisibility, is_ganger_visible};
