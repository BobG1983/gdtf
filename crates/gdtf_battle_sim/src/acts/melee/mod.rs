//! The **melee** dispatch — drains a buffered [`MeleeRequested`](crate::acts::request::MeleeRequested), gates 8-adjacency (+ clear
//! LOS + an alive opposing target on the ganger arm), spends the wielded melee weapon's primary
//! fight-mode TU, and branches on the [`MeleeTarget`](crate::acts::request::MeleeTarget): the §7 opposed-Fight → §5 damage → §6
//! wound synthesis onto a ganger (GTW-507), or the UNCONTESTED cover-smash onto an adjacent
//! inert STRUCTURE (GTW-508, child GTW-37d of the GTW-37 melee epic; `docs/combat/resolution.md`
//! §7).
//!
//! No combat math is reimplemented here: the ganger path REUSES the GTW-506 opposed-Fight
//! core ([`opposed_fight`](crate::melee::opposed_fight) /
//! [`melee_damage_mult`](crate::melee::melee_damage_mult) /
//! [`apply_melee_multiplier`](crate::melee::apply_melee_multiplier)) and the existing §4/§5/§6
//! pieces verbatim, composed by [`resolve_melee_strike`](crate::melee::resolve_melee_strike);
//! the structural path REUSES [`resolve_structural_melee`](crate::melee::resolve_structural_melee)
//! (multiplied weapon damage through the EXISTING cover ledger). The gates REUSE
//! [`is_8_adjacent`](crate::acts::downed::is_8_adjacent) and [`has_los`](crate::los::has_los)
//! verbatim. Param-only (`bevy-traps.md` #7 — no `&mut World`); the disjoint queries coexist
//! with no `B0001` conflict (see the per-query docs).
//!
//! ## Module layout (GTW-508 C6 / GTW-583 — code-health size cap)
//!
//! Wiring-only `mod.rs`; every concern lives in a focused submodule:
//!
//! - `queries` — the query `type` aliases + the [`MeleeGrids`] / [`MeleeRngs`] /
//!   [`MeleeFacts`] [`SystemParam`](bevy::ecs::system::SystemParam) bundles;
//! - `dispatch` — the [`dispatch_melee`] system (drain → snapshot → weapon resolve →
//!   arm branch);
//! - `snapshot` — the shared attacker snapshot + stream-borrow bundles the two arms take;
//! - `ganger` — the contested §7 opposed-Fight arm (`resolve_ganger_melee`): gate
//!   8-adjacency + opposing faction + alive + LOS, spend the fight-mode TU, run the
//!   §7 → §5 → §6 synthesis onto the target, emit the connect signals;
//! - `structure` — the UNCONTESTED GTW-508 cover-smash arm (`resolve_structure_melee`):
//!   gate ONLY 8-adjacency, spend the same TU, apply multiplied (`mult_max`) weapon
//!   damage through the EXISTING cover ledger — NO opposed roll, NO RNG draw.

mod dispatch;
mod ganger;
mod queries;
mod snapshot;
mod structure;

pub use dispatch::dispatch_melee;
pub(super) use queries::{MeleeFacts, MeleeGrids, MeleeRngs};
