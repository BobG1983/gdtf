//! The **melee weapon model** — the GTW-505 (child GTW-37a of the GTW-37 melee epic)
//! sibling to the ranged weapon model, sharing the ranged DAMAGE model while dropping
//! ranged-only handling and adding melee-only mechanics.
//!
//! A melee weapon is an entity related to its ganger via the SAME
//! [`Wields`](super::Wields) relationship the ranged weapon uses (ADR-0004): every
//! ganger wields BOTH a ranged weapon entity (carrying the [`Weapon`](super::Weapon)
//! marker) AND a melee weapon entity (carrying the [`MeleeWeapon`] marker), so any
//! ganger can melee (the GTW-37 D3 ruling — an authored melee weapon OR the
//! [`fists`](registry::FISTS_KEY) default). The two markers let a ranged-weapon lookup
//! EXCLUDE melee weapons ([`Wields::ranged_weapon`](super::Wields::ranged_weapon)) so
//! relating a melee weapon NEVER regresses ranged firing (GTW-505 C5).
//!
//! ## What it SHARES with the ranged model
//!
//! The melee weapon REUSES the ranged DAMAGE newtypes verbatim — the
//! [`WeaponName`](super::WeaponName), the [`WeaponDamage`](super::WeaponDamage) /
//! [`WeaponPunch`](super::WeaponPunch) / [`WeaponShred`](super::WeaponShred) /
//! [`DamageType`](super::DamageType) damage group, the [`FatalBias`](super::FatalBias),
//! and the [`Handedness`](super::Handedness). The weapon MULTIPLIES damage only (no
//! Fight-roll bonus) — the GTW-37 first-slice ruling.
//!
//! ## What it DROPS / ADDS
//!
//! DROPS every ranged-only handling field — `base_spread` / `accuracy` / `kickback` /
//! the [`Magazine`](crate::magazine::Magazine) / the [`Stable`](super::Stable) tag (a
//! melee strike has no cone, no ammo, no brace). ADDS the melee-only [`Reach`] (default
//! `1`) + the [`FightMode`] selector (the ranged [`FireMode`](super::FireMode) mirror,
//! MINUS the dispersion `cone_mult`: a per-mode flat [`TuCost`] + [`Strikes`] count).
//!
//! GTW-505 is the MODEL + ASSET-LAYOUT + SPAWN child only. The opposed-Fight resolution
//! math (GTW-506) and the melee ACT / input / presenter (GTW-507) CONSUME these types;
//! nothing here computes a melee strike.
//!
//! Code-health: this concern is a dir-module split by responsibility mirroring the
//! ranged layout — the melee-only components ([`components`]), the fight-mode model
//! ([`fight_mode`]), the spawn bundle ([`bundle`]), the authoring spec ([`spec`]), and
//! the registry ([`registry`]). This `mod.rs` is wiring-only; every public path is
//! preserved via the re-exports below.

mod bundle;
mod components;
mod fight_mode;
mod registry;
mod spec;

pub use bundle::{MeleeDamageProfile, MeleeWeaponBundle};
pub use components::{MeleeWeapon, Reach};
pub use fight_mode::{FightMode, FightModeKind, FightModeSpec, Strikes, TuCost};
pub use registry::{FISTS_KEY, MeleeWeaponRegistry};
pub use spec::MeleeWeaponSpec;
