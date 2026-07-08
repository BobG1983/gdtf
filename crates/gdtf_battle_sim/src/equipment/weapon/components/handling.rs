//! Capacity / identity / handedness — [`MagazineSize`], [`WeaponName`], and
//! [`Handedness`]: who fires it, what it's called, how many rounds.

use bevy::prelude::{Component, Deref};
use serde::{Deserialize, Serialize};

/// A weapon's **magazine size** — how many rounds it holds before a reload
/// (resolution.md §"What's tunable" names the `reload_tu` refill; the magazine's
/// capacity is a weapon number). The ammo clamp `fire()` honors (resolution.md
/// §"What's pure math vs sim": "ammo clamp") reads this.
///
/// A weapon NUMBER, a small non-negative count (`u16`). Private inner + derived
/// [`Deref`]; `#[serde(transparent)]` ([`Serialize`] so the editor's ATTACHMENT mode
/// saves an `ExtraAmmo` payload in the same schema it loads — GTW-669). Since GTW-275
/// it is **not** a standalone
/// `#[derive(Component)]` — it is the `size` LEAF of the [`crate::magazine::Magazine`]
/// grouping component (the user's `Magazine { size, reload_tu, … }` model), which
/// also carries the per-weapon [`ReloadTu`](crate::magazine::ReloadTu) and the live
/// [`LoadedRounds`](crate::magazine::LoadedRounds) battle-state count.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct MagazineSize(u16);

impl MagazineSize {
    /// Build a magazine size from its round count.
    #[must_use]
    pub const fn new(rounds: u16) -> Self {
        Self(rounds)
    }

    /// The capacity as its inner round count — a `const` accessor (the derived
    /// [`Deref`] is not `const`, so `const fn` callers like
    /// [`Magazine::loaded`](crate::magazine::Magazine::loaded) read the capacity
    /// through this).
    #[must_use]
    pub const fn get(self) -> u16 {
        self.0
    }
}

/// A weapon's **name** — its human-facing identity (e.g. an authored weapon's
/// display name). Carried on the armed entity for the weapon display / picker
/// (GTW-254) and the loader (GTW-257); the §1/§6 cone/severity math NEVER reads it,
/// so it is deliberately absent from the [`WeaponStats`](crate::weapon::WeaponStats)
/// borrow-view.
///
/// A weapon-identity newtype over [`String`] (no-bare-types: a name is a domain
/// value). Private inner + derived [`Deref`]; `#[serde(transparent)]` parses a bare
/// RON string. A `#[derive(Component)]` (GTW-200) — its OWN sibling component on the
/// armed entity, NOT packed into the [`Weapon`](super::markers::Weapon) unit marker. (A fire mode has no
/// stored name — its label is [`ModeKind`](crate::weapon::ModeKind)'s [`Display`](std::fmt::Display),
/// GTW-260.)
/// `Default` (`WeaponName(String::new())`, the empty string) is a **spawn-seed
/// sentinel only** — the `bsn!` spawn path seeds the slot via `Default` before
/// `WeaponName::new(..)` overwrites it (GTW-322). It is NOT a valid authored
/// weapon name.
#[derive(Component, Deref, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct WeaponName(String);

impl WeaponName {
    /// Build a weapon name from its display string.
    #[must_use]
    pub const fn new(name: String) -> Self {
        Self(name)
    }
}

/// A weapon's **handedness** — how many hands it takes to fire (GTW-443). A
/// [`OneHanded`](Handedness::OneHanded) weapon (a pistol) can be fired with a single
/// working hand; a [`TwoHanded`](Handedness::TwoHanded) weapon (a long-arm or heavy
/// piece) needs BOTH hands and is refused once a hand-disabling injury leaves the
/// shooter with fewer than two (see [`HandsAvailable`](crate::injuries::HandsAvailable)
/// and the shared `can_fire` guard).
///
/// A named domain enum (no-bare-types-exempt: an enum carries its meaning in its
/// variants, not a wrapped primitive). It lives as its OWN sibling
/// `#[derive(Component)]` newtype on the armed entity (GTW-200), parsed from the
/// `handedness:` field of a weapon's `.weapon.ron`. `Copy` + `Hash` + `Eq` so it can be
/// a value field of the [`FireActor`](crate::magazine::FireActor) read-bundle.
/// `Default` ([`OneHanded`](Handedness::OneHanded)) is a **spawn-seed sentinel only** —
/// the `bsn!` spawn path seeds the slot via `Default` before the authored
/// `Handedness::<Variant>` patch overwrites it (GTW-322). It carries no special meaning
/// as the default; `OneHanded` is chosen as the least-restrictive node so an
/// un-authored weapon never spuriously gates on hand count.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Default)]
pub enum Handedness {
    /// A one-handed weapon — fires with a single working hand (a pistol).
    #[default]
    OneHanded,
    /// A two-handed weapon — needs BOTH hands; refused at fewer than two available
    /// hands (a long-arm or heavy piece).
    TwoHanded,
}
