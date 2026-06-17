//! The E4.5 **`fire()` volley orchestration** — the firing-act integrator, LAST by
//! dependency (`docs/combat/resolution.md` §1 / §1a / §"What's pure math vs sim":
//! "`fire()` owns the economy; every draw from the model RNG").
//!
//! [`fire`] runs the whole firing act over **two disjoint Bevy queries** — NO
//! `&mut World` exclusive (the GTW-198 user direction; the GTW-200 weapon
//! decomposition makes the shooter's full state — ganger state + each weapon stat
//! [`Component`](bevy::prelude::Component) — gettable from ONE query, and the struck
//! target gettable from a SECOND query via `get_mut(entity)` where the entity rides
//! out of [`ShotKind::Ganger`](crate::resolve_coarse::ShotKind::Ganger)). The act, in
//! order:
//!
//! 1. **Validate** via the shared E4.4 [`can_fire`](crate::magazine::can_fire) guard
//!    (the [`FireActor`](crate::magazine::FireActor) assembled from the queried
//!    components, the shooter's [`LifeState`](crate::ganger::LifeState) read from the
//!    TARGET query). If it fails → an **empty** volley, mutating NOTHING (no TU
//!    charge, no draw) — fail-closed (AC2).
//! 2. **Charge** the full mode TU ONCE up front via E4.0
//!    [`spend_tu`](crate::tu::spend_tu)
//!    ([`mode_tu_cost`](crate::magazine::mode_tu_cost) = `ModeTuPercent × TuMax ×
//!    the ×1.5 aim premium when aiming`), regardless of how many rounds the burst
//!    loops (AC3).
//! 3. **Clamp** the burst to ammo ([`clamp_burst`](crate::magazine::clamp_burst) =
//!    `min(ModeShots, Magazine rounds)`); the [`Magazine`](crate::magazine::Magazine)
//!    decrements one round per fired iteration (saturating, AC4).
//! 4. **Per-round loop** `i in 0..clamped`: compose a
//!    [`ShotInputs`](crate::resolve_coarse::ShotInputs) with EVERY field (shooter
//!    pos/facing/stance + target pos/stance + the target cell's cover band + `cone` =
//!    E4.3 [`cone_for`](crate::aim::cone_for) at `prior_shots = i` + `p` = E2.5
//!    [`concentration_p`](crate::sample_cone::concentration_p) + `prior_shots =
//!    PriorShots::new(i)` + `recoil_climb = tuning.cone_stability.recoil_climb` +
//!    `recoil_growth` from [`stability_for`](crate::aim::stability_for)), run E2
//!    [`resolve_coarse`](crate::resolve_coarse::resolve_coarse), and — only on a
//!    [`ShotKind::Ganger`](crate::resolve_coarse::ShotKind::Ganger) — fold E3
//!    [`resolve_and_apply`](crate::resolve_and_apply::resolve_and_apply) onto the
//!    struck target (got from the target query); every non-ganger kind folds to
//!    [`HitReport::no_effect`](crate::resolve_and_apply::HitReport::no_effect) (AC5 /
//!    AC6). The recoil climbs across the burst and RESETS between `fire()` calls
//!    (each `fire()` starts at `prior_shots = 0`).
//! 5. **Freeze** — returns the `Vec<HitReport>` volley (one report per fired round).
//!
//! ## The two-query disjoint-access design (AC1)
//!
//! The two queries share **no mutable component**, so they coexist without a Bevy
//! `B0001` access conflict: the SHOOTER query holds the read stats + `&mut Tu` +
//! `&mut Magazine` (and **no** `&LifeState`); the TARGET query holds `&mut Hp` /
//! `&mut Wounds` / `&mut LifeState` / `&mut WornArmor` + `&Toughness` + `&Luck`.
//! [`Luck`](crate::ganger::Luck) is read-only in BOTH (a `&`-vs-`&` overlap is
//! compatible — only a write-vs-read/write of the SAME component conflicts). The
//! shooter's own [`LifeState`](crate::ganger::LifeState) is read from the TARGET query
//! (the shooter is also a ganger → `targets.get(shooter)`), so `&LifeState` never
//! enters the shooter query (which would clash with the target query's `&mut
//! LifeState`).
//!
//! Render-free, deterministic model logic: every random draw bottoms out in the
//! single injected [`SimRng`](crate::rng::SimRng) (no `thread_rng`, no ad-hoc
//! entropy), so the same [`BattleSeed`](crate::rng::BattleSeed) reproduces a
//! byte-equal volley (AC7); no LOS / fog input is consulted (the presenter boundary).
//! **Zero pixels** — the reports carry only damage / wound math, never a screen
//! coordinate.

mod compose;
mod query;
mod volley;

pub use query::{BattleGrids, FireOrder, ShooterQuery, TargetQuery};
pub use volley::fire;

#[cfg(test)]
mod test;
