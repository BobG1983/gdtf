//! The id vocabulary the WIRE itself still owns — the grid coordinates and the capture
//! file stem (GTW-734, narrowed by GTW-943).
//!
//! Two concerns, one file each: the grid coordinate wire types ([`cell`]) and the capture
//! [`shot`] name a [`CaptureRider`](crate::command::CaptureRider) carries. Every type is a
//! private-inner newtype with a derived `Deref` and a `new` constructor (no-bare-types
//! rules 1–5), serde-transparent so it rides the wire as its bare inner.
//!
//! # Why these two stayed when the rest moved
//!
//! GTW-943 moved the entity tokens, the pointer position and the misc handles into the GAME
//! host's own `wire/` module: they described the old per-request vocabulary, and what
//! replaces it is per-host command arguments. These two did not move because they are
//! named HERE, in shapes this crate owns and every peer decodes: [`ShotName`] is a field of
//! [`CaptureRider`](crate::command::CaptureRider), and the [`cell`] coordinates are the
//! shared grid vocabulary a host's command arguments embed — which is what the optional
//! `schema` feature exists to derive `JsonSchema` over.

pub mod cell;
pub mod shot;

pub use cell::{CellLevelNet, CellNet, CellXNet, CellYNet, LevelNet};
pub use shot::ShotName;

#[cfg(test)]
mod test;
