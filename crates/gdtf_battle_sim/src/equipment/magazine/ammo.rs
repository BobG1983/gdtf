//! The battle-local ammo STATE: the [`Magazine`] grouping component (the weapon's
//! [`MagazineSize`] capacity + its per-weapon [`ReloadTu`] reload cost + the
//! [`LoadedRounds`] currently loaded), its saturating per-round decrement, its
//! reload refill, and the [`clamp_burst`] burst-clamp primitive.

use bevy::prelude::{Component, Deref};
use serde::{Deserialize, Serialize};

use crate::weapon::{AmmoType, MagazineSize, ModeShots};

/// A magazine's **per-weapon reload cost** — the flat number of Time Units a ganger
/// spends to reload this weapon (the user's `Magazine { size, reload_tu, … }` model,
/// USER DIRECTIVE 2026-06-17: "have it cost some number of TU set by the weapons
/// definition").
///
/// The cost the GTW-275 `dispatch_reload` charges via [`crate::tu::spend_tu`] — a
/// per-WEAPON number authored in the weapon `.ron`, NOT a global tuning leaf. A small
/// `u8` count, matching [`crate::ganger::Tu`]'s inner type (and the
/// [`StanceChangeTu`](crate::tuning::StanceChangeTu) / [`TurnTu`](crate::tuning::TurnTu)
/// per-act cost shape) so the economy subtracts it directly. Magnitudes are tunable
/// balance DATA (commented in the `.ron`), NOT pinned by tests. Private inner + derived
/// [`Deref`]; `#[serde(transparent)]` parses a bare RON scalar ([`Serialize`] so the
/// editor's WEAPON mode saves the field in the same schema it loads — GTW-670).
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ReloadTu(u8);

impl ReloadTu {
    /// Build a per-weapon reload TU cost from its flat Time-Unit magnitude (tunable
    /// balance data).
    #[must_use]
    pub const fn new(tu: u8) -> Self {
        Self(tu)
    }
}

/// The **rounds currently loaded** in a magazine — the battle-local round count that
/// [`fire`](crate::fire::fire) decrements and a reload refills.
///
/// The live ammo state (was the old standalone `Magazine(u16)` count, now the
/// [`Magazine`] grouping's loaded-rounds leaf). A `u16` count matching
/// [`MagazineSize`](crate::weapon::MagazineSize), clamped at construction so it never
/// exceeds the capacity. A named newtype (no-bare-types: a round count is a domain
/// value): private inner + derived [`Deref`]; `#[serde(transparent)]` lets it default
/// from / parse a bare RON scalar.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Deserialize)]
#[serde(transparent)]
pub struct LoadedRounds(u16);

impl LoadedRounds {
    /// Build a loaded-rounds count from its round number (the caller clamps to the
    /// magazine's [`MagazineSize`](crate::weapon::MagazineSize) capacity).
    #[must_use]
    pub const fn new(rounds: u16) -> Self {
        Self(rounds)
    }
}

/// Whether a [`Magazine`] holds **no rounds** — the `can_fire` ammo gate's answer.
///
/// A named predicate newtype (no-bare-types: "the magazine is empty" is a domain answer,
/// not a bare `bool`) over [`Magazine::is_empty`]. Private inner, read through the derived
/// [`Deref`].
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct MagazineEmpty(bool);

/// Whether a [`Magazine`] is loaded to **its full capacity** — the reload act's
/// no-op-refill answer.
///
/// A named predicate newtype (no-bare-types: "the magazine is full" is a domain answer,
/// not a bare `bool`) over [`Magazine::is_full`]. Private inner, read through the derived
/// [`Deref`].
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct MagazineFull(bool);

/// A ganger's **magazine** — the weapon's round capacity, its per-weapon reload cost,
/// and the rounds currently loaded, grouped into one Component (the user's
/// `Magazine { size, reload_tu, … }` model, USER DIRECTIVE 2026-06-17).
///
/// A GROUPING component (the [`FireModeSpec`](crate::weapon::FireModeSpec) precedent:
/// related data travels as one named record with named-newtype leaves, no [`Deref`] on
/// the grouping itself). It carries:
///
/// - [`size`](Magazine::size) — the weapon's [`MagazineSize`] capacity (was a separate
///   sibling weapon Component before GTW-275);
/// - [`reload_tu`](Magazine::reload_tu) — the per-weapon [`ReloadTu`] cost of a reload;
/// - [`rounds`](Magazine::rounds) — the [`LoadedRounds`] currently loaded (was the old
///   standalone `Magazine(u16)`), clamped at construction so it never exceeds `size`;
/// - [`loaded_ammo`](Magazine::loaded_ammo) — the [`AmmoType`] currently loaded (GTW-775),
///   the class the compatibility gate ([`ammo_compatible`]) matches a candidate against.
///
/// [`spend_round`](Magazine::spend_round) drains it one round at a time (saturating);
/// [`refill`](Magazine::refill) tops it back to `size` (the reload act);
/// [`load`](Magazine::load) is the compatibility-gated sibling that may swap in a
/// (weapon-accepted) ammo class; [`clamp_burst`] reads it to bound a burst's shot count.
/// Build it with [`Magazine::loaded`] / [`Magazine::loaded_with`] (full, the latter
/// choosing the loaded [`AmmoType`]) or [`Magazine::new`] (a partial load, clamped). The
/// authoring `.ron` deserializes the static config (`size` + `reload_tu`) via the
/// derived [`Deserialize`]; [`WeaponSpec::into_bundle`](crate::weapon::WeaponSpec::into_bundle)
/// spawns it FULL.
///
/// Defaults to `0` rounds, size `0`, reload `0`, and [`AmmoType::Slug`] loaded (empty — a
/// structural spawn default, not a balance value; a fresh ganger carries no ammo until the
/// situation setup loads a magazine from the resolved weapon bundle).
///
/// [`Serialize`] (GTW-670) so the editor's WEAPON mode saves the authored grouping in
/// the same schema it loads — with `rounds` `#[serde(skip_serializing)]`: the live
/// loaded count is SPAWN-ONLY, never authored, so a saved `.weapon.ron` writes exactly
/// `(size, reload_tu)` like every hand-authored member.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Magazine {
    /// The weapon's round capacity — the maximum [`rounds`](Magazine::rounds) it can
    /// hold. Authored per-weapon.
    pub size:        MagazineSize,
    /// The per-weapon TU cost of a reload (the GTW-275 `dispatch_reload` charge).
    /// Authored per-weapon.
    pub reload_tu:   ReloadTu,
    /// The rounds currently loaded — the live ammo state `fire()` decrements and a
    /// reload refills. Not authored in the `.ron` (defaults to `0` on deserialize and
    /// is skipped on serialize — GTW-670); spawned FULL by
    /// [`WeaponSpec::into_bundle`](crate::weapon::WeaponSpec::into_bundle).
    #[serde(default, skip_serializing)]
    pub rounds:      LoadedRounds,
    /// The **ammunition class currently loaded** (GTW-775) — the [`AmmoType`] a
    /// compatible reload / [`load`](Magazine::load) tops up, and the identity the
    /// [`ammo_compatible`] gate matches a candidate against. Runtime state, NOT
    /// authored: it defaults to [`AmmoType::Slug`] on deserialize and is skipped on
    /// serialize (mirroring `rounds`), so a `magazine: (size, reload_tu)` block
    /// parses unchanged; [`WeaponSpec::into_bundle`](crate::weapon::WeaponSpec::into_bundle)
    /// spawns it as the weapon's ACCEPTED class via [`Magazine::loaded_with`].
    #[serde(default, skip_serializing)]
    pub loaded_ammo: AmmoType,
}

impl Magazine {
    /// Build a magazine loaded with `rounds`, **clamped** to `size`, carrying the
    /// weapon's `reload_tu` (the GTW-275 grouping ctor).
    ///
    /// The general constructor: a request above capacity is clamped down to capacity
    /// (`min(rounds, *size)`) — the magazine can never hold more than its weapon's size
    /// (AC1) — and a request within capacity is preserved exactly. Use
    /// [`loaded`](Magazine::loaded) for a full magazine.
    #[must_use]
    pub const fn new(rounds: LoadedRounds, size: MagazineSize, reload_tu: ReloadTu) -> Self {
        Self {
            size,
            reload_tu,
            rounds: LoadedRounds(if rounds.0 < size.get() {
                rounds.0
            } else {
                size.get()
            }),
            // The loaded ammo class is the spawn-seed default here; the weapon's ACCEPTED
            // class is threaded in at spawn by `loaded_with` (via `into_bundle`).
            loaded_ammo: AmmoType::Slug,
        }
    }

    /// Build a **full** magazine — loaded to `size`, carrying the weapon's `reload_tu`,
    /// with the spawn-seed [`AmmoType::Slug`] loaded.
    ///
    /// The convenience constructor for a freshly-reloaded weapon at full capacity
    /// ([`new`](Magazine::new) with `rounds == *size`). Use
    /// [`loaded_with`](Magazine::loaded_with) to choose the loaded [`AmmoType`] — the
    /// GTW-275 spawn-full path
    /// ([`WeaponSpec::into_bundle`](crate::weapon::WeaponSpec::into_bundle) uses it to
    /// load the weapon's ACCEPTED class, GTW-775).
    #[must_use]
    pub const fn loaded(size: MagazineSize, reload_tu: ReloadTu) -> Self {
        Self::loaded_with(size, reload_tu, AmmoType::Slug)
    }

    /// Build a **full** magazine loaded with a chosen ammo class (GTW-775) — loaded to
    /// `size`, carrying the weapon's `reload_tu`, with `ammo` as the loaded
    /// [`AmmoType`].
    ///
    /// The GTW-775 spawn-full path: [`WeaponSpec::into_bundle`](crate::weapon::WeaponSpec::into_bundle)
    /// calls this with the weapon's [`accepts`](crate::weapon::WeaponSpec::accepts) class,
    /// so a freshly-spawned weapon starts loaded with the ammo it accepts (and the
    /// [`ammo_compatible`] gate holds for a reload of the same class).
    #[must_use]
    pub const fn loaded_with(size: MagazineSize, reload_tu: ReloadTu, ammo: AmmoType) -> Self {
        Self {
            size,
            reload_tu,
            rounds: LoadedRounds(size.get()),
            loaded_ammo: ammo,
        }
    }

    /// The rounds **currently loaded** — the live ammo count.
    #[must_use]
    pub const fn rounds(&self) -> LoadedRounds {
        self.rounds
    }

    /// The magazine's **capacity** — the [`MagazineSize`] the loaded count is clamped
    /// to.
    #[must_use]
    pub const fn size(&self) -> MagazineSize {
        self.size
    }

    /// The magazine's **per-weapon reload cost** — the [`ReloadTu`] the reload act
    /// charges.
    #[must_use]
    pub const fn reload_tu(&self) -> ReloadTu {
        self.reload_tu
    }

    /// The ammunition class **currently loaded** (GTW-775) — the [`AmmoType`] the
    /// compatibility gate ([`ammo_compatible`]) matches a candidate against.
    #[must_use]
    pub const fn loaded_ammo(&self) -> AmmoType {
        self.loaded_ammo
    }

    /// Whether the magazine is **empty** — no rounds loaded (the `can_fire` ammo gate).
    #[must_use]
    pub const fn is_empty(&self) -> MagazineEmpty {
        MagazineEmpty(self.rounds.0 == 0)
    }

    /// Whether the magazine is **full** — loaded to its capacity (the reload act reads
    /// it to decide whether a reload is a no-op refill).
    #[must_use]
    pub const fn is_full(&self) -> MagazineFull {
        MagazineFull(self.rounds.0 >= self.size.get())
    }

    /// Spend **one** round — a **saturating** decrement that floors at `0`.
    ///
    /// The per-round primitive the E4.5 `fire()` burst loop calls once per fired
    /// round (resolution.md §"What's pure math vs sim": `fire()` "per-round spend").
    /// Uses [`u16::saturating_sub`]: spending a round from an already-empty
    /// magazine leaves it at `0` — **never** underflows / wraps to `~65535` (AC2).
    pub const fn spend_round(&mut self) {
        self.rounds.0 = self.rounds.0.saturating_sub(1);
    }

    /// **Refill** the magazine to full — set the loaded rounds equal to `size`.
    ///
    /// The reload primitive the GTW-275 `dispatch_reload` runs on a successful, paid
    /// reload (resolution.md L166 names the `reload_tu` refill). Idempotent: reloading
    /// an already-full magazine leaves it full. Tops up the CURRENTLY loaded
    /// [`AmmoType`](Magazine::loaded_ammo) — it never changes the loaded class (that is
    /// [`load`](Magazine::load)'s job).
    pub const fn refill(&mut self) {
        self.rounds.0 = self.size.get();
    }

    /// **Load** a candidate ammo class, GATED on the weapon's accepted class — the
    /// compatibility gate beside [`refill`](Magazine::refill) (GTW-775, child GTW-400).
    ///
    /// A magazine may only take on ammo its weapon accepts: on a `candidate` whose class
    /// matches `accepted` ([`ammo_compatible`]) this sets the loaded class to `candidate`
    /// and [`refill`](Magazine::refill)s to full; on an INCOMPATIBLE candidate it leaves
    /// the magazine entirely unchanged — neither the loaded class nor the round count
    /// moves. Returns whether the load happened ([`AmmoCompatible`]).
    ///
    /// This is ADDITIVE and does NOT rewrite the existing primitives: [`refill`](Magazine::refill)
    /// (which tops up the already-loaded class) and the shared
    /// [`can_fire`](super::guard::can_fire) guard are unchanged. The reload act (GTW-275)
    /// still refills unconditionally — this gate concerns ammo TYPE identity, not
    /// availability (there is no ammo reserve; the scarcity mechanic is the deferred
    /// campaign-layer sibling GTW-776). `load` is the type-gated entry a future
    /// ammo-swap act calls.
    #[must_use]
    pub fn load(&mut self, candidate: AmmoType, accepted: AmmoType) -> AmmoCompatible {
        let compatible = ammo_compatible(accepted, candidate);
        if *compatible {
            self.loaded_ammo = candidate;
            self.refill();
        }
        compatible
    }
}

/// Whether a `candidate` ammo class is **compatible** with the class a weapon
/// `accepts` — the compatibility rule the [`Magazine::load`] gate reads (GTW-775,
/// child GTW-400).
///
/// The rule is class IDENTITY: a weapon takes ONLY its accepted munition class
/// (`candidate == accepted`). This is the pure gate beside the magazine's
/// [`refill`](Magazine::refill) / [`can_fire`](super::guard::can_fire) guard — no
/// [`World`](bevy::ecs::world::World) access, deterministic, unit-testable.
#[must_use]
pub fn ammo_compatible(accepted: AmmoType, candidate: AmmoType) -> AmmoCompatible {
    AmmoCompatible(accepted == candidate)
}

/// Whether a candidate ammo class is compatible with the weapon's accepted class —
/// the answer [`ammo_compatible`] / [`Magazine::load`] return (GTW-775).
///
/// A named predicate newtype (no-bare-types: "the ammo is compatible" is a domain
/// answer, not a bare `bool`). Private inner, read through the derived [`Deref`].
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct AmmoCompatible(bool);

/// Clamp a fire mode's shot count to the rounds actually in the magazine — the
/// burst-clamp primitive the E4.5 `fire()` burst loop runs.
///
/// `fire()` can never loop more rounds than are loaded (resolution.md §"What's pure
/// math vs sim": the "ammo clamp"): the looped count is `min(mode shots, rounds
/// left)`. Returns a [`ModeShots`](crate::weapon::ModeShots) so the bounded count
/// keeps its per-mode meaning. With an empty magazine the result is `0` (no round
/// fires); within ammo the mode's full shot count passes through unchanged.
#[must_use]
pub fn clamp_burst(shots: ModeShots, mag: &Magazine) -> ModeShots {
    ModeShots::new((*shots).min(*mag.rounds()))
}
