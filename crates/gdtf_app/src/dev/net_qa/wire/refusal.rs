//! Why the sim turned an act down, one reason type per act.

use gdtf_battle_input::ShotRefusal;
use gdtf_battle_sim::{
    acts::ReloadOutcome,
    posture::{FacingRefusal, StanceRefusal},
};
use serde::{Deserialize, Serialize};

/// Why the sim will not take a shot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ShotRefusalNet {
    /// The entity holds none of the parts a shooter needs.
    NotAShooter,
    /// The shooter wields no weapon that fires.
    NoFiringWeapon,
    /// The shooter is not alive.
    NotAlive,
    /// The shooter cannot afford the fire mode's time units.
    Unaffordable,
    /// The magazine holds no rounds.
    MagazineEmpty,
    /// The target cell-level is off the battle grid.
    OutOfBounds,
    /// Too few hands are free for the weapon.
    NotEnoughHands,
}

impl ShotRefusalNet {
    /// Mirror the game's own reason.
    #[must_use]
    pub const fn from_sim(refusal: ShotRefusal) -> Self {
        match refusal {
            ShotRefusal::NotAShooter => Self::NotAShooter,
            ShotRefusal::NoFiringWeapon => Self::NoFiringWeapon,
            ShotRefusal::NotAlive => Self::NotAlive,
            ShotRefusal::Unaffordable => Self::Unaffordable,
            ShotRefusal::MagazineEmpty => Self::MagazineEmpty,
            ShotRefusal::OutOfBounds => Self::OutOfBounds,
            ShotRefusal::NotEnoughHands => Self::NotEnoughHands,
        }
    }
}

/// Why the sim will not reload.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ReloadRefusalNet {
    /// The magazine is already full.
    AlreadyFull,
    /// The shooter cannot afford the reload.
    NoTu,
}

impl ReloadRefusalNet {
    /// Mirror the sim's outcome, or nothing when the magazine was refilled.
    #[must_use]
    pub const fn from_sim(outcome: ReloadOutcome) -> Option<Self> {
        match outcome {
            ReloadOutcome::Reloaded => None,
            ReloadOutcome::AlreadyFull => Some(Self::AlreadyFull),
            ReloadOutcome::NoTu => Some(Self::NoTu),
        }
    }
}

/// Why the sim will not change stance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StanceRefusalNet {
    /// The actor already holds that stance.
    AlreadyHeld,
    /// The actor cannot afford the change.
    Unaffordable,
}

impl StanceRefusalNet {
    /// Mirror the sim's own reason.
    #[must_use]
    pub const fn from_sim(refusal: StanceRefusal) -> Self {
        match refusal {
            StanceRefusal::AlreadyHeld => Self::AlreadyHeld,
            StanceRefusal::Unaffordable => Self::Unaffordable,
        }
    }
}

/// Why the sim will not turn the actor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FacingRefusalNet {
    /// The actor already faces that way.
    AlreadyFacing,
    /// The pool cannot pay for even one ring step.
    Unaffordable,
}

impl FacingRefusalNet {
    /// Mirror the sim's own reason.
    #[must_use]
    pub const fn from_sim(refusal: FacingRefusal) -> Self {
        match refusal {
            FacingRefusal::AlreadyFacing => Self::AlreadyFacing,
            FacingRefusal::Unaffordable => Self::Unaffordable,
        }
    }
}
