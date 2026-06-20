//! Armor: the four per-location armor stats, an [`ArmorPiece`] bundling them for
//! one body location, the read-only roster [`SourceArmor`] record, and the
//! battle-local [`WornArmor`] component that mid-battle wear mutates.
//!
//! This is the E1.3 battle-local-armor slice. The armor model is shared by
//! gangers and cover (`docs/combat/resolution.md` §3) and feeds the per-hit
//! formula in `docs/combat/weapons-and-armor.md` §"Per-hit resolution":
//!
//! 1. `effPen = max(0, punch − hardness)`
//! 2. `dmg    = max(floor, damage − max(0, protection − effPen))`
//! 3. `integrity −= min(protection, damage) + effPen + shred` — useless at `≤ 0`
//!
//! The four armor fields are each a named newtype (no-bare-types) over `i32`:
//! the formula subtracts and clamps these against weapon stats, and `integrity`
//! must track **below zero** ("useless at `≤ 0`"), so a *signed* integer is the
//! honest inner type — it never underflows on the `protection − effPen` and
//! `integrity − wear` subtractions the way an unsigned type would. The magnitudes
//! are TBD tuning (`weapons-and-armor.md` §"TBD"); only the *mechanism* is built
//! and tested here. Per `weapons-and-armor.md`, **hardness does not degrade** —
//! only [`ArmorIntegrity`] wears, so it is the worn copy's single mutable field.
//!
//! The model/view split (ADR-0001, `docs/decisions/0001-rust-bevy-rewrite.md`): the
//! [`SourceArmor`] record is
//! the roster representation and is **never** mutated during a battle; a battle
//! starts by seeding an owned [`WornArmor`] copy **by value** from it, and only
//! that copy wears. Seeding by value means a worn-copy mutation can never leak
//! back to the roster source. The full situation→entities setup that places these
//! components on ganger entities is E1.8 (GTW-158), not this slice.
//!
//! ## E3.1 — the armor-type vocabulary
//!
//! The E3.1 slice adds [`ArmorType`] (the armor half of the dual vocabulary of
//! `docs/combat/matchup.md` §"The 7 types") and gives every [`ArmorPiece`] an
//! `armor_type` so a struck piece carries its wheel node. [`ArmorType`]'s seven
//! variants name the same seven wheel nodes as [`crate::weapon::DamageType`], node
//! `i` of one mirroring node `i` of the other — one tournament wheel drives both
//! sides. This is the DATA substrate only; the matchup lookup (the wheel itself)
//! is a later E3 slice, not here.
//!
//! ## GTW-269 — the data-driven armor loader model
//!
//! The GTW-269 slice adds the authoring + registry types that mirror the landed
//! weapon model (GTW-257): [`ArmorSpec`] (the per-armor `.ron` authoring struct, the
//! armor mirror of [`crate::weapon::WeaponSpec`]) and the [`ArmorRegistry`] /
//! [`ArmorName`] name→spec map (the armor mirror of
//! [`crate::weapon::WeaponRegistry`] / [`crate::weapon::WeaponName`]). This slice
//! ADDS those types only — they are not yet consumed by [`WornArmor::seed_from`] /
//! `GangerSpawn` / [`SourceArmor`] (that consumption swap is a later slice), so the
//! existing roster/battle-local model is unchanged.
//!
//! GTW-201 code-health: this concern is a dir-module split by responsibility — the
//! per-location stats / keys / wheel node ([`stats`]), the roster / battle-local
//! armor records ([`worn`]), the authoring spec ([`spec`]), and the registry
//! ([`registry`]). This `mod.rs` is wiring-only; every public path is preserved via
//! the re-exports below.

mod registry;
mod spec;
mod stats;
mod worn;

#[cfg(test)]
mod test;

pub use registry::{ArmorName, ArmorRegistry};
pub use spec::ArmorSpec;
pub use stats::{
    ArmorFloor, ArmorHardness, ArmorIntegrity, ArmorPiece, ArmorProtection, ArmorType, BodyPart,
};
pub use worn::{SourceArmor, SourceArmorDef, WornArmor};
