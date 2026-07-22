//! The throw-grenade act request + resolved signal — [`ThrowGrenadeRequested`] and
//! [`ThrowResolved`].

use bevy::prelude::{Entity, Message};

use crate::{metric::CellLevel, weapon::DamageType};

/// A **throw-grenade** act was requested — `thrower` lobs its
/// [`TrajectoryStyle::Arc`](crate::weapon::TrajectoryStyle) grenade at the `target` cell
/// (GTW-546, child GTW-41d).
///
/// A buffered [`Message`] (`bevy-traps.md` #4 — NOT the observer `Event`) carrying the
/// throwing ganger [`Entity`] + the target [`CellLevel`] the grenade is lobbed at. The
/// player-only contextual Throw button writes this from the input queue (the GTW-571 per-act
/// contextual queue -> `ThrowGrenadeRequested`) when the selected ganger wields an
/// `Arc` weapon. The throw is BLIND — there is NO line-of-sight / facing / arc gate (a lob
/// need not see its target), so the target payload is a CELL AT RANGE (a [`CellLevel`], like
/// [`MoveRequested::dest`](super::movement::MoveRequested::dest)), never an 8-adjacent entity.
/// [`dispatch_throw_grenade`](crate::acts::throw_grenade::dispatch_throw_grenade) drains it and
/// RE-GATES in the sim: the thrower exists + wields an `Arc` weapon with a loaded round + can
/// afford the [`ThrowTu`](crate::tuning::ThrowTu) leaf. On pass it spends that leaf + one
/// magazine round, marches the deterministic arc ([`march_arc`](crate::march::march_arc)) —
/// blocked by an intact roof, passing holes / windows — and fans the weapon's
/// [`HitType::Blast`](crate::weapon::HitType) at the landing through the GTW-541 resolver.
///
/// The `thrower` is a Bevy [`Entity`] handle — framework plumbing, the only bare type the
/// no-bare-types rule permits in a payload; `target` is the landed [`CellLevel`] newtype.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ThrowGrenadeRequested {
    /// The throwing ganger — the [`ThrowTu`](crate::tuning::ThrowTu) leaf + one magazine
    /// round are spent off it, and its wielded `Arc` weapon supplies the blast stats.
    pub thrower: Entity,
    /// The target `(cell, level)` the grenade is lobbed at — the arc march's aim; the blast
    /// fans from the arc's landing (the target cell, unless an intact roof intercepts).
    pub target:  CellLevel,
}

impl ThrowGrenadeRequested {
    /// Build a throw-grenade request for `thrower` lobbing at the `target` cell.
    #[must_use]
    pub const fn new(thrower: Entity, target: CellLevel) -> Self {
        Self { thrower, target }
    }
}

/// A **throw-grenade** act RESOLVED — the presenter-facing output signal that a lobbed grenade
/// landed at `at`, carrying the weapon's `damage` type for the impact FX (GTW-546).
///
/// A buffered [`Message`] (`bevy-traps.md` #4 — NOT the observer `Event`) emitted by
/// [`dispatch_throw_grenade`](crate::acts::throw_grenade::dispatch_throw_grenade) once per resolved
/// throw (at the arc's LANDING cell — the target cell, or the roof-block cell if an intact
/// roof intercepted). It mirrors [`MeleeResolved`](super::melee::MeleeResolved) / [`ShotFired`](crate::shot_fired::ShotFired):
/// it carries ONLY what the presenter's impact / blast FX needs — the landing cell
/// ([`CellLevel`]) and the grenade's [`DamageType`] (the FX color/role selector) — never combat
/// math. The presenter reads it through a [`MessageReader`](bevy::prelude::MessageReader) (the
/// one-way sim → presenter dep). The blast's HP/wound mutations are applied to the struck
/// gangers' components and observed via change-detection; this signal is the dedicated
/// *grenade-landed* moment (the app + presenter phase draws it).
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ThrowResolved {
    /// The `(cell, level)` the grenade landed at — the arc's landing (the impact / blast FX
    /// draws here). A [`CellLevel`] newtype, never a bare `IVec3`.
    pub at:     CellLevel,
    /// The grenade's [`DamageType`] — the FX role/color selector (the impact glyph picks its
    /// tint/tile from this, the way [`MeleeResolved`](super::melee::MeleeResolved) does). A domain enum, never a bare label.
    pub damage: DamageType,
}

impl ThrowResolved {
    /// Build a throw-resolved signal for a grenade landing at `at` with the weapon's `damage`
    /// type.
    #[must_use]
    pub const fn new(at: CellLevel, damage: DamageType) -> Self {
        Self { at, damage }
    }
}
