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
//! replaces it is per-host command arguments. [`ShotName`] did not move because it is named
//! HERE, in a shape this crate owns and every peer decodes: it is a field of
//! [`CaptureRider`](crate::command::CaptureRider).
//!
//! The [`cell`] coordinates stayed for a reason that no longer holds. GTW-943 kept them as
//! "the shared grid vocabulary a host's command arguments embed"; GTW-944 then minted the
//! game host's own copy in `gdtf_app`'s `wire/cell.rs`, because its command arguments must
//! use types that crate owns (a foreign type cannot gain `JsonSchema` there). So the game
//! host embeds its copy, not this one, and this one's only remaining users are inside
//! `gdtf_qa_command`: the `FakeCell` test-support command every host's conformance suite
//! runs, and two of that crate's own tests. Whether it keeps a home here or moves into
//! `gdtf_qa_command`'s test support is an open call — deciding it is not GTW-944's, whose
//! contract only mandates the game-side copy.

pub mod cell;
pub mod shot;

pub use cell::{CellLevelNet, CellNet, CellXNet, CellYNet, LevelNet};
pub use shot::ShotName;

#[cfg(test)]
mod test;
