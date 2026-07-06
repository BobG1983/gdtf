//! The melee act request + its output facts — [`MeleeTarget`], [`MeleeRequested`],
//! [`MeleeResolved`], and [`MeleeStruck`].

use bevy::prelude::{Entity, Message};

use crate::{metric::CellLevel, resolve_hit::HpDamage, weapon::DamageType};

/// What a [`MeleeRequested`] strike is aimed at — an opposing GANGER, or an adjacent inert
/// STRUCTURE (a Cover / Wall cell) (GTW-508, child GTW-37d of GTW-37).
///
/// A named domain enum (no-bare-types: a melee target is a domain value, not a bare
/// `Entity`-or-`CellLevel` union). The shared act-intent seam routes a structural-melee
/// target the SAME way it routes a ganger target — a melee intent carries either an
/// enemy-actor target ([`Ganger`](Self::Ganger)) or an adjacent-structure cell target
/// ([`Structure`](Self::Structure)); [`dispatch_melee`](crate::acts::melee::dispatch_melee)
/// branches on this. The [`Ganger`](Self::Ganger) arm runs the GTW-506/507 opposed-Fight
/// path (unchanged); the [`Structure`](Self::Structure) arm runs the GTW-508 UNCONTESTED
/// cover-smash (no opposed roll, no [`FightRng`](crate::rng::FightRng) draw).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MeleeTarget {
    /// An opposing GANGER (the GTW-507 contested path) — the strike runs the §7
    /// opposed-Fight → §5 damage → §6 wound synthesis against this alive, 8-adjacent,
    /// in-LOS enemy. The [`Entity`] is framework plumbing (the only bare type the
    /// no-bare-types rule permits in a payload).
    Ganger(Entity),
    /// An adjacent inert STRUCTURE at this `(cell, level)` (the GTW-508 uncontested
    /// cover-smash) — a Cover or Wall cell the strike smashes with multiplied damage
    /// through the cover ledger, NO opposed roll. A [`CellLevel`] newtype, never a bare
    /// `IVec3`.
    Structure(CellLevel),
}

/// A **melee** act was requested — `attacker` strikes `target` in close combat (GTW-507,
/// child GTW-37c; the adjacent-structure target added by GTW-508, child GTW-37d).
///
/// A buffered [`Message`] (`bevy-traps.md` #4 — NOT the observer `Event`) carrying the
/// attacking ganger [`Entity`] plus the [`MeleeTarget`] the strike is aimed at — an
/// opposing GANGER (the GTW-507 contested path) OR an adjacent inert STRUCTURE cell (the
/// GTW-508 uncontested cover-smash). Everything the ganger-vs-ganger verb reads (each
/// ganger's [`Fight`](crate::ganger::Fight), the wielded melee weapon's stats, the
/// defender's stance / armor) lives on the entities; the structural path reads the struck
/// cover's HP + armor from the [`CoverLedger`](crate::cover::CoverLedger) at the target
/// cell — so the message needs only the attacker + the target.
/// [`dispatch_melee`](crate::acts::melee::dispatch_melee) drains it, spends the wielded melee
/// weapon's primary fight-mode TU, and — per [`MeleeTarget`] — runs the §7 opposed-Fight →
/// §5 damage → §6 wound pipeline (ganger) or the uncontested §5 damage × `mult_max` →
/// `deplete_cover` cover-smash (structure). The `attacker` is a Bevy [`Entity`] handle —
/// framework plumbing, the only bare type the no-bare-types rule permits in a payload.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MeleeRequested {
    /// The attacking (striking) ganger — its [`Fight`](crate::ganger::Fight) + wielded melee
    /// weapon resolve the blow.
    pub attacker: Entity,
    /// What the strike is aimed at — an opposing ganger or an adjacent structure cell.
    pub target:   MeleeTarget,
}

impl MeleeRequested {
    /// Build a melee request for `attacker` striking the opposing GANGER `target` (the
    /// GTW-507 contested path) — the ganger-vs-ganger form.
    #[must_use]
    pub const fn new(attacker: Entity, target: Entity) -> Self {
        Self {
            attacker,
            target: MeleeTarget::Ganger(target),
        }
    }

    /// Build a melee request for `attacker` smashing the adjacent STRUCTURE at `at` (the
    /// GTW-508 uncontested cover-smash) — the melee-vs-structure form.
    #[must_use]
    pub const fn new_structural(attacker: Entity, at: CellLevel) -> Self {
        Self {
            attacker,
            target: MeleeTarget::Structure(at),
        }
    }
}

/// A **melee** act RESOLVED — the presenter-facing output signal that a melee strike landed at
/// `at`, carrying the weapon's `damage` type for the strike FX (GTW-507).
///
/// A buffered [`Message`] (`bevy-traps.md` #4 — NOT the observer `Event`) emitted by
/// [`dispatch_melee`](crate::acts::melee::dispatch_melee) once per CONNECTING melee hit (a missed
/// opposed roll deals no damage and emits nothing). It mirrors the structural output signals
/// ([`crate::occupancy_sync::CoverDestroyed`] / [`ShotFired`](crate::shot_fired::ShotFired)):
/// it carries ONLY what the presenter's strike-glyph FX needs — the target cell the strike
/// landed at ([`CellLevel`]) and the wielded melee weapon's [`DamageType`] (the FX color/role
/// selector) — never combat math. The presenter reads it through a
/// [`MessageReader`](bevy::prelude::MessageReader) (the one-way sim → presenter dep; the sim
/// never reads the presenter). The HP/wound mutations themselves are applied to the target's
/// components and observed by the presenter via change-detection + the existing wound/injury
/// signals; this signal is the dedicated *strike-landed* moment.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MeleeResolved {
    /// The `(cell, level)` the strike landed at — the struck target's cell (where the strike
    /// glyph draws). A [`CellLevel`] newtype, never a bare `IVec3`.
    pub at:     CellLevel,
    /// The wielded melee weapon's [`DamageType`] — the FX role/color selector (the strike
    /// glyph picks its tint/tile from this, the way [`ShotFired`](crate::shot_fired::ShotFired)
    /// does). A domain enum, never a bare label.
    pub damage: DamageType,
}

impl MeleeResolved {
    /// Build a melee-resolved signal for a connecting strike at `at` with the weapon's
    /// `damage` type.
    #[must_use]
    pub const fn new(at: CellLevel, damage: DamageType) -> Self {
        Self { at, damage }
    }
}

/// A ganger was **struck in melee** — a connecting §7 strike applied `hp_damage` from
/// `attacker` onto `target` (GTW-572).
///
/// The NUMBER-BEARING melee fact the combat log's melee-damage line reads. The sibling
/// [`MeleeResolved`] stays the presenter strike-GLYPH signal (cell + damage type only — the
/// FX contract, GTW-507); this fact carries the two combatant [`Entity`] handles (each
/// resolved to a display name at the presenter boundary) plus the applied
/// [`HpDamage`](crate::resolve_hit::HpDamage), which the strike verb previously computed
/// and dropped internally. Emitted ONCE per CONNECTING ganger strike by
/// [`resolve_ganger_melee`](crate::acts::melee::dispatch_melee)'s ganger arm; a miss, or a
/// structural cover-smash, emits nothing (no ganger took damage). The sim emits the FACT —
/// the amount it applied — never a rendered line (the presenter phrases it, ADR-0001).
///
/// A buffered [`Message`] (`bevy-traps.md` #4 — NOT the observer `Event`), mirroring
/// [`MeleeResolved`]. The `attacker` / `target` are Bevy [`Entity`] handles — framework
/// plumbing, the only bare type the no-bare-types rule permits in a payload.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MeleeStruck {
    /// The striking ganger — the melee-damage line's subject.
    pub attacker:  Entity,
    /// The struck ganger — the melee-damage line's object.
    pub target:    Entity,
    /// The §5→§7-multiplied HP loss the connecting strike applied onto the target.
    pub hp_damage: HpDamage,
}

impl MeleeStruck {
    /// Build a melee-struck fact: `attacker` applied `hp_damage` onto `target`.
    #[must_use]
    pub const fn new(attacker: Entity, target: Entity, hp_damage: HpDamage) -> Self {
        Self {
            attacker,
            target,
            hp_damage,
        }
    }
}
