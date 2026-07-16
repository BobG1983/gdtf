//! [`LifeStateNet`] — the wire mirror of the sim two-pool life-state machine
//! (GTW-734).

use serde::{Deserialize, Serialize};

/// A ganger's terminal **life state** — the wire mirror of the sim `LifeState`.
///
/// The two-pool outcome: [`Alive`](Self::Alive) (up and fighting), [`Downed`](Self::Downed)
/// (HP gone, Wounds remaining — incapacitated but alive, bleeding out unless stabilized),
/// [`Dead`](Self::Dead) (Wounds gone). An independent serde enum; the bleed-out clock is a
/// separate sim marker (`BleedingOut`, GTW-695) not folded into `LifeState`, so it is NOT
/// a variant here — a QA client reads down-ness from [`Downed`](Self::Downed).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LifeStateNet {
    /// Up and fighting — full agency (the sim's `Alive`).
    Alive,
    /// HP gone, Wounds remaining — incapacitated but alive (the sim's `Downed`).
    Downed,
    /// Wounds gone — dead in battle (the sim's `Dead`).
    Dead,
}
