//! The [`NetIntent`](super::NetIntent) payload enums — the posture / melee-target wire
//! mirrors (GTW-734).
//!
//! Independent serde enums that mirror the sim's `StanceKind`, aim flag, `Direction`,
//! and `MeleeTarget` — never a leak of the sim types (this crate is bevy-free). Kept
//! out of the main [`net_intent`](super::net_intent) file (module-layout: one concern
//! per file).

use bevy_derive::Deref;
use serde::{Deserialize, Serialize};

use crate::ids::{CellLevelNet, GangerToken};

/// A wire posture — the mirror of the sim `StanceKind` a
/// [`NetIntent::SetStance`](super::NetIntent::SetStance) sets DIRECTLY.
///
/// The three postures a ganger can hold. An independent serde enum (the sim's
/// `Standing` / `Crouching` / `Prone`); the game side maps it back to `StanceKind`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StanceNet {
    /// Upright (the sim's `Standing`).
    Standing,
    /// Kneeling (the sim's `Crouching`).
    Crouching,
    /// Flat (the sim's `Prone`).
    Prone,
}

/// A requested **aim mode** — the mirror of the sim `AimRequest` a
/// [`NetIntent::SetAiming`](super::NetIntent::SetAiming) sets.
///
/// A private-inner newtype over `bool` (no-bare-types: `true` = aimed fire, `false` =
/// hip-fired — the `SetAimingRequested::aim` flag), serde-transparent. The input crate
/// exposes this as a TOGGLE (`ActIntent::AimToggle`); the wire mirrors the sim's
/// explicit `SetAimingRequested` aim flag, so the game side maps a requested aim onto
/// the toggle (reading the ganger's current aim to decide).
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AimNet(bool);

impl AimNet {
    /// Build a requested aim mode — `true` to aim, `false` to hip-fire.
    #[must_use]
    pub const fn new(aim: bool) -> Self {
        Self(aim)
    }
}

/// A wire facing — the mirror of the sim 8-way `Direction` a
/// [`NetIntent::SetFacing`](super::NetIntent::SetFacing) turns to.
///
/// The square grid's 8-way compass (cardinals + diagonals). An independent serde enum
/// carrying the same eight variants in the sim's ring order (`North` = 0 clockwise to
/// `NorthWest` = 7); the game side maps it back to `Direction` and emits a
/// `SetFacingRequested` (the input crate's `ActIntent::Turn`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FacingNet {
    /// Toward −Y.
    North,
    /// Toward +X, −Y.
    NorthEast,
    /// Toward +X.
    East,
    /// Toward +X, +Y.
    SouthEast,
    /// Toward +Y.
    South,
    /// Toward −X, +Y.
    SouthWest,
    /// Toward −X.
    West,
    /// Toward −X, −Y.
    NorthWest,
}

/// A wire melee target — the mirror of the sim `MeleeTarget` a
/// [`NetIntent::Melee`](super::NetIntent::Melee) strikes.
///
/// The ONE melee intent routes two target kinds (the sim's `MeleeTarget`): a contested
/// strike on an opposing [`Ganger`](MeleeTargetNet::Ganger), or an uncontested smash of
/// an adjacent inert [`Structure`](MeleeTargetNet::Structure) cover / wall cell. An
/// independent serde enum; the game side maps it back to `MeleeTarget`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MeleeTargetNet {
    /// An opposing ganger (the contested path), by its wire token.
    Ganger(GangerToken),
    /// An adjacent inert structure cell (the cover-smash), by its `(cell, level)` key.
    Structure(CellLevelNet),
}
