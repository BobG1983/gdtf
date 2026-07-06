//! Weapon-side data — the per-weapon and per-fire-mode NUMBERS the §1
//! cone/stability/recoil/aim math reads (`docs/combat/resolution.md` §1(a)+(b),
//! §"Coefficients live in the combat-tuning data": "Weapon-side numbers
//! (`base_spread`, `accuracy`, `kickback`, `fatal_bias`, `magazine_size`; per-mode
//! cone mult / TU% / shots) live on the weapon / fire-mode data").
//!
//! This is the home conflict's other side: *coefficients* live in [`crate::tuning`],
//! but a *weapon's own numbers* live here on [`Weapon`] / [`FireMode`]. The cone
//! equation `θ_cone = base_spread × stability × aim × firemode × recoil`
//! (resolution.md §1a) reads a weapon's `base_spread` and per-round `kickback`
//! from [`Weapon`] and its selector term from a [`FireMode`]; the in-cone draw
//! `p = concentration_p(Shooting, weapon.accuracy)` (§1b) reads `accuracy`; and
//! the §6 severity score reads `fatal_bias` (carried here, consumed by E3 — not
//! used in this slice).
//!
//! This is the E2.1 **data/types/serde** slice: types + serde only, no math or
//! behavior. Every numeric leaf is a named newtype (no-bare-types: a private inner
//! value, a derived [`Deref`](bevy::prelude::Deref), and `#[serde(transparent)]` so
//! it round-trips as a bare RON scalar), matching the [`crate::tuning`] house style.
//!
//! ## E3.1 — damage stats + the damage-type vocabulary
//!
//! The E3.1 slice adds the three per-hit damage NUMBERS the
//! `docs/combat/weapons-and-armor.md` §"Weapon stats" / §"Per-hit resolution"
//! formula reads — [`WeaponDamage`], [`WeaponPunch`], [`WeaponShred`] — plus the
//! [`DamageType`] a weapon emits. [`DamageType`] is one half of the dual
//! vocabulary of `docs/combat/matchup.md` §"The 7 types": its seven variants name
//! the same seven wheel nodes as [`crate::armor::ArmorType`], node `i` of one
//! mirroring node `i` of the other. This is the DATA substrate only — the per-hit
//! formula and the matchup lookup (the wheel itself) are later E3 slices; nothing
//! here computes a hit or a matchup.
//!
//! ## GTW-200 — a weapon is ECS components, not a data struct
//!
//! The user-corrected model: a weapon is **not** one packed data struct — each
//! sub-value is its own `#[derive(Component)]` newtype that lives as a sibling
//! component on the (ganger) entity, and [`Weapon`] is a unit MARKER component
//! (no data). The stats are then queryable directly, so the combat act
//! (E4.5 `fire()`) is a proper query-based Bevy system with NO `&mut World`
//! indirection. [`WeaponBundle`] spawns an armed entity carrying the full set;
//! the §1/§6 readers take a transient [`WeaponStats`] borrow-view (refs assembled
//! from the components at the call site — NOT a stored component).
//!
//! GTW-201 code-health: this concern is a dir-module split by responsibility —
//! the per-stat components (`components`), the fire-mode model (`fire_mode`),
//! the spawn bundle + borrow-view (`bundle`), the authoring spec (`spec`), and
//! the registry (`registry`). This `mod.rs` is wiring-only; every public path is
//! preserved via the re-exports below.

mod bundle;
mod components;
mod dot;
mod fire_mode;
mod melee;
mod registry;
mod relationship;
mod silenced;
mod spec;
mod trajectory;

#[cfg(test)]
mod test;

// GTW-624: attachment types are NOT weapon API. The effect PALETTE lives in
// `crate::effects::attachments` and the MECHANICS (registry / spec / key / commands
// extension / slot-gated fit) in `crate::equipment::attachments` — import them from
// those true paths. This mod.rs re-exports ONLY its own descendants, keeping the
// GTW-558 palette/mechanics split visible at the public surface.
pub use bundle::{DamageProfile, HandlingProfile, WeaponBundle, WeaponStats};
pub use components::{
    Accuracy, BaseSpread, DamageType, FatalBias, Handedness, Kickback, MagazineSize, MountedWeapon,
    Shove, Silenced, Stable, Weapon, WeaponDamage, WeaponName, WeaponPunch, WeaponShred,
};
// GTW-544 (child GTW-41e): the damage-over-time model — the weapon-side `DotProfile` a DOT
// weapon authors + the per-ganger `Dot` affliction a penetrating hit attaches. The runtime
// per-turn drain is `effects::dot::tick_dot`; this module owns only the data types.
pub use dot::{Dot, DotDamage, DotProfile, DotTurns};
pub use fire_mode::{
    AoeRange, BlastRadius, ConeHalfAngle, FireMode, FireModeSpec, HitType, ModeConeMult, ModeKind,
    ModeShots, ModeTuPercent,
};
// GTW-505 (child GTW-37a): the melee weapon model — the sibling to the ranged model
// above, sharing the ranged damage newtypes (re-exported here) while adding the
// melee-only Reach / FightMode + the MeleeWeapon marker. See the `melee` module doc.
pub use melee::{
    FISTS_KEY, FightMode, FightModeKind, FightModeSpec, MeleeDamageProfile, MeleeWeapon,
    MeleeWeaponBundle, MeleeWeaponRegistry, MeleeWeaponSpec, Reach, Strikes, TuCost,
};
pub use registry::WeaponRegistry;
pub use relationship::{WieldedBy, Wields};
// GTW-549: the shared silenced-weapon gate — relocated out of the ripped-out GTW-542
// `attachment` module; the `Silenced` component (above) is fitted by the `Silence`
// attachment effect, and BOTH loud-signal producers key off this predicate.
pub use silenced::shooter_weapon_silenced;
pub use spec::{PendingAttachments, WeaponSpawnSiblings, WeaponSpec};
// GTW-546 (child GTW-41d): the per-weapon trajectory style — a `Straight` ray (the
// default, every existing weapon) or a lobbed `Arc` (a grenade / grenade launcher). The
// fire path reads it to pick the straight `march_vector` or the parabolic `march_arc`.
pub use trajectory::TrajectoryStyle;
