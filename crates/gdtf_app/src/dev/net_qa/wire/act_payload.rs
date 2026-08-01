//! The [`NetIntent`](super::act::NetIntent) payload enums — the posture / melee-target wire
//! mirrors (GTW-734).
//!
//! Independent serde enums that mirror the sim's `StanceKind`, aim flag, `Direction` and
//! `MeleeTarget` — never a re-export of the sim types, so the wire shape is this module's
//! to keep stable and a sim rename is a compile error here rather than a silent wire
//! change. Kept out of [`act`](super::act) itself (module-layout: one concern per file).
//!
//! A caller CHOOSES every value here — a posture, an aim flag, a facing, which of the two
//! melee targets — so none of them has a publishing read. The C10 `act.set_stance` /
//! `act.set_aiming` / `act.set_facing` and `act.melee` are what take them; the C6
//! `battle.roster` card reports a ganger's CURRENT stance and facing, which is how a
//! caller knows what it is changing from.

use bevy::prelude::Deref;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{cell::CellLevelNet, token::GangerToken};

/// A wire posture — the mirror of the sim `StanceKind` a
/// [`NetIntent::SetStance`](super::act::NetIntent::SetStance) sets DIRECTLY.
///
/// The three postures a ganger can hold. An independent serde enum (the sim's
/// `Standing` / `Crouching` / `Prone`); the game side maps it back to `StanceKind`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub enum StanceNet {
    /// Upright (the sim's `Standing`).
    Standing,
    /// Kneeling (the sim's `Crouching`).
    Crouching,
    /// Flat (the sim's `Prone`).
    Prone,
}

/// A requested **aim mode** — the mirror of the sim `AimRequest` a
/// [`NetIntent::SetAiming`](super::act::NetIntent::SetAiming) sets.
///
/// A private-inner newtype over `bool` (no-bare-types: `true` = aimed fire, `false` =
/// hip-fired — the `SetAimingRequested::aim` flag), serde-transparent. The input crate
/// exposes this as a TOGGLE (`ActIntent::AimToggle`); the wire mirrors the sim's
/// explicit `SetAimingRequested` aim flag, so the game side maps a requested aim onto
/// the toggle (reading the ganger's current aim to decide).
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
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
/// [`NetIntent::SetFacing`](super::act::NetIntent::SetFacing) turns to.
///
/// The square grid's 8-way compass (cardinals + diagonals). An independent serde enum
/// carrying the same eight variants in the sim's ring order (`North` = 0 clockwise to
/// `NorthWest` = 7); the game side maps it back to `Direction` and emits a
/// `SetFacingRequested` (the input crate's `ActIntent::Turn`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
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
/// [`NetIntent::Melee`](super::act::NetIntent::Melee) strikes.
///
/// The ONE melee intent routes two target kinds (the sim's `MeleeTarget`): a contested
/// strike on an opposing [`Ganger`](MeleeTargetNet::Ganger), or an uncontested smash of
/// an adjacent inert [`Structure`](MeleeTargetNet::Structure) cover / wall cell. An
/// independent serde enum; the game side maps it back to `MeleeTarget`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub enum MeleeTargetNet {
    /// An opposing ganger (the contested path), by its wire token.
    Ganger(GangerToken),
    /// An adjacent inert structure cell (the cover-smash), by its `(cell, level)` key.
    Structure(CellLevelNet),
}
