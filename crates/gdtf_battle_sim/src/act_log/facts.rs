//! The AFTER-value snapshots the query-sourced recorders carry on their deeds — the
//! posture, vitals and magazine states a mutating act LEFT BEHIND (GTW-727 C4).
//!
//! Every mutating [`ActDeed`](super::ActDeed) carries the after value of what it changed,
//! so a consumer APPLIES the recorded value and never re-derives it from live world state
//! (which is exactly the "outcome shown before its cause" defect this log exists to cure).
//! Three of those after-values are multi-field states with no single owning message —
//! posture, vitals, magazine — so each gets one named snapshot struct here, recorded from
//! a change-detected transition rather than a signal.

use bevy::prelude::Deref;

use crate::{
    combatants::ganger::{Aiming, Facing, Hp, Position, Stance, Tu, Wounds},
    inflicted_wound::InflictedWounds,
    injuries::InflictedInjuries,
    magazine::Magazine,
};

/// Whether a ganger is currently SUPPRESSED (GTW-526).
///
/// The sim carries suppression as the PRESENCE of a
/// [`Suppressed`](crate::ganger::Suppressed) component, so the recorded after-value is
/// that presence flattened to a named flag — never a bare `bool` (`no-bare-types.md`).
/// Flattening it is what lets a consumer treat suppression CLEARING (a component removal,
/// which `Changed<T>` cannot observe) as an ordinary field transition.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct SuppressedNow(bool);

impl SuppressedNow {
    /// Build a suppression flag from whether the ganger currently carries
    /// [`Suppressed`](crate::ganger::Suppressed).
    #[must_use]
    pub const fn new(suppressed: bool) -> Self {
        Self(suppressed)
    }

    /// Whether the ganger is suppressed — the `const`-callable read the derived [`Deref`]
    /// cannot give.
    #[must_use]
    pub const fn is_suppressed(self) -> bool {
        self.0
    }
}

/// A ganger's drawn POSTURE after a posture-changing act — facing, stance, aim, and the
/// suppression flag, together.
///
/// One snapshot rather than four deeds: the four fields decide ONE thing (how the ganger
/// is standing), a single act commonly moves more than one of them at once (a suppression
/// application also drops the stance), and a consumer that mirrors posture wants the
/// settled combination, not a stream of partial edits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PoseFacts {
    /// The direction the ganger faces after the act.
    pub facing:     Facing,
    /// The ganger's stance after the act.
    pub stance:     Stance,
    /// Whether the ganger is aiming after the act.
    pub aiming:     Aiming,
    /// Whether the ganger is suppressed after the act.
    pub suppressed: SuppressedNow,
}

impl PoseFacts {
    /// Build a posture snapshot from its four settled parts.
    #[must_use]
    pub const fn new(
        facing: Facing,
        stance: Stance,
        aiming: Aiming,
        suppressed: SuppressedNow,
    ) -> Self {
        Self {
            facing,
            stance,
            aiming,
            suppressed,
        }
    }
}

/// A ganger's VITALS after a damaging (or restoring) act — the five values the status /
/// inspect stat block renders.
///
/// Recorded as ONE snapshot because damage arrives through many paths that share no
/// message: a shot's in-fold apply, a melee strike, a fall, a bleed / DOT / field tick.
/// Carrying the settled after-values on one deed means a consumer applies exactly what the
/// sim reached, in log order, without knowing which path produced it.
///
/// NOT `Copy` — [`inflicted`](Self::inflicted) and [`injuries`](Self::injuries) are owned
/// lists.
#[derive(Debug, Clone, PartialEq)]
pub struct VitalsFacts {
    /// The ganger's time units after the act.
    pub tu:        Tu,
    /// The ganger's hit points after the act.
    pub hp:        Hp,
    /// The ganger's remaining wounds pool after the act.
    pub wounds:    Wounds,
    /// The ganger's inflicted-wound list after the act.
    pub inflicted: InflictedWounds,
    /// The ganger's durable injury ledger after the act.
    pub injuries:  InflictedInjuries,
}

impl VitalsFacts {
    /// Build a vitals snapshot from its five settled parts.
    #[must_use]
    pub const fn new(
        tu: Tu,
        hp: Hp,
        wounds: Wounds,
        inflicted: InflictedWounds,
        injuries: InflictedInjuries,
    ) -> Self {
        Self {
            tu,
            hp,
            wounds,
            inflicted,
            injuries,
        }
    }
}

/// A weapon's MAGAZINE after a round left it (or a reload refilled it).
///
/// Carried on its own deed, addressed to the WEAPON entity (not the wielding ganger), so a
/// consumer mirroring the ammo readout applies it to the same entity the panel reads.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MagazineFacts(Magazine);

impl MagazineFacts {
    /// Build a magazine snapshot from the weapon's settled magazine.
    #[must_use]
    pub const fn new(magazine: Magazine) -> Self {
        Self(magazine)
    }

    /// The wrapped magazine — the `const`-callable read the derived [`Deref`] cannot give.
    #[must_use]
    pub const fn inner(self) -> Magazine {
        self.0
    }
}

/// A ganger's POSITION after a step — the settled `(cell, level)` the walk left it on.
///
/// [`MovementOccurred`](crate::acts::MovementOccurred) carries only the ground
/// [`Cell`](crate::metric::Cell) pair, which cannot express a cross-storey step, so the
/// movement recorder reads the settled [`Position`] and carries it here as the step's
/// after-value.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PositionFacts(Position);

impl PositionFacts {
    /// Build a position snapshot from the ganger's settled position.
    #[must_use]
    pub const fn new(position: Position) -> Self {
        Self(position)
    }

    /// The wrapped position — the `const`-callable read the derived [`Deref`] cannot give.
    #[must_use]
    pub const fn inner(self) -> Position {
        self.0
    }
}
