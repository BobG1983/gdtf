//! The grid-coordinate wire types — [`CellXNet`] / [`CellYNet`] / [`LevelNet`] and the
//! composed [`CellNet`] / [`CellLevelNet`] keys (GTW-734, minted for this host by GTW-944).
//!
//! The wire mirror of the sim's `Cell` (an `IVec2` ground cell), `Level` (a `u8` storey
//! index, `0..MAX_LEVELS`) and `CellLevel` (the `(cell, level)` key). Independent serde
//! types — never a leak of the glam-backed sim types — so a QA client parses them without
//! `bevy_math`. Each scalar is its own newtype (no-bare-types rule 3: an x is not a y is
//! not a storey).
//!
//! # Where a caller gets one
//!
//! A cell coordinate is the one wire value a client may FABRICATE: it is a grid position,
//! not a handle, so "the cell two east of my ganger" is a legal thing to compute and send.
//! Reads publish them all the same — the C6 `battle.roster` card carries each ganger's
//! `(cell, storey)`, C7's `battle.inspect` / `battle.visible` are addressed by one, and the
//! C10 `act.move` / `act.fire` and C14 `input.click_cell` take one as their target.

use bevy::prelude::Deref;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A cell's ground-plane **x** grid coordinate, in cell units — the wire mirror of the sim
/// `Cell`'s x axis.
///
/// A private-inner newtype (no-bare-types), serde-transparent so it rides the wire as its
/// bare `i32`. Signed: the sim floors negative positions into negative cells, so a
/// coordinate can be negative.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct CellXNet(i32);

impl CellXNet {
    /// Build a cell x-coordinate from its grid value.
    #[must_use]
    pub const fn new(x: i32) -> Self {
        Self(x)
    }
}

/// A cell's ground-plane **y** grid coordinate, in cell units — the wire mirror of the sim
/// `Cell`'s y axis.
///
/// A private-inner newtype (no-bare-types), serde-transparent. Distinct from [`CellXNet`]
/// by rule 3 even though both wrap `i32`.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct CellYNet(i32);

impl CellYNet {
    /// Build a cell y-coordinate from its grid value.
    #[must_use]
    pub const fn new(y: i32) -> Self {
        Self(y)
    }
}

/// A 0-based **storey** index — which floor of the coarse grid, valid `0..MAX_LEVELS` — the
/// wire mirror of the sim `Level`.
///
/// A private-inner newtype (no-bare-types), serde-transparent so it rides the wire as its
/// bare `u8`. The contract does not re-encode the `MAX_LEVELS` bound here; the game side
/// validates against the live grid extent. A caller reads a storey off any positional read
/// (the C6 roster card, C7's `battle.visible`) or counts from the one the C9 `view.read`
/// reports as active.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct LevelNet(u8);

impl LevelNet {
    /// Build a storey index from its floor number.
    #[must_use]
    pub const fn new(storey: u8) -> Self {
        Self(storey)
    }
}

/// A 2D ground-plane cell key — the `(x, y)` pair, the wire mirror of the sim `Cell`.
///
/// A named-field struct (not a bare tuple) so each coordinate keeps its meaning; its two
/// fields are the typed [`CellXNet`] / [`CellYNet`] scalars. `deny_unknown_fields`, so the
/// derived schema publishes `additionalProperties: false` and a misspelt field is a
/// decode error rather than a silently dropped coordinate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CellNet {
    /// The cell's ground-plane x coordinate.
    pub x: CellXNet,
    /// The cell's ground-plane y coordinate.
    pub y: CellYNet,
}

impl CellNet {
    /// Build a ground cell from its `x`/`y` grid coordinates.
    #[must_use]
    pub const fn new(x: CellXNet, y: CellYNet) -> Self {
        Self { x, y }
    }
}

/// The 3D grid key — a [`CellNet`] paired with its [`LevelNet`] storey, the wire mirror of
/// the sim `CellLevel` `(cell, level)` identity.
///
/// The key a ganger position / impact cell lives at, and the target the C10 `act.move` /
/// `act.fire` / `act.throw_grenade` and the C12 cell-addressed contextual acts take.
/// `deny_unknown_fields`, like [`CellNet`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CellLevelNet {
    /// The ground-plane cell.
    pub cell:  CellNet,
    /// The storey the cell sits on.
    pub level: LevelNet,
}

impl CellLevelNet {
    /// Build a `(cell, level)` key from a ground cell and its storey.
    #[must_use]
    pub const fn new(cell: CellNet, level: LevelNet) -> Self {
        Self { cell, level }
    }
}
