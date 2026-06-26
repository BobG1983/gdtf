//! Armor: the four per-location armor stats, an [`ArmorPiece`] bundling them for
//! one body location, the read-only roster [`SourceArmor`] record, and the
//! battle-local armor-piece **entities** (related to a ganger via [`Wears`]) whose
//! [`ArmorIntegrity`] mid-battle wear mutates.
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
//! [`SourceArmor`] record is the roster representation and is **never** mutated during a
//! battle; a battle starts by spawning per-piece armor **entities** (related to the
//! ganger via [`Wears`], ADR-0004) seeded **by value** from a registry-resolved
//! [`ArmorSpec`], and only those battle-local entities wear. Seeding by value means a
//! worn mutation can never leak back to the roster source. The situation→entities setup
//! that spawns and relates these piece entities is `setup_battle`
//! (`crate::situation::setup_battle`).
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
//! [`crate::weapon::WeaponRegistry`] / [`crate::weapon::WeaponName`]). Since GTW-269 the
//! [`ArmorRegistry`] resolves each ganger's authored armor key into the [`ArmorSpec`]
//! that `setup_battle` spawns the per-piece armor entities from (the `GangerSpawn`
//! authors only the key; [`SourceArmor`] remains the roster authoring shape).
//!
//! GTW-201 code-health: this concern is a dir-module split by responsibility — the
//! per-location stats / keys / wheel node ([`stats`]), the roster / battle-local
//! armor records ([`worn`]), the authoring spec ([`spec`]), and the registry
//! ([`registry`]). This `mod.rs` is wiring-only; every public path is preserved via
//! the re-exports below.

mod registry;
mod relationship;
mod spec;
mod stats;
mod worn;

#[cfg(test)]
mod test;

pub use registry::{ArmorName, ArmorRegistry};
pub use relationship::{PieceArmorMut, Wears, WornBy};
pub use spec::ArmorSpec;
pub use stats::{
    ArmorFloor, ArmorHardness, ArmorIntegrity, ArmorPiece, ArmorProtection, ArmorType, BodyPart,
    InjuryCategory,
};
pub use worn::{SourceArmor, SourceArmorDef};
