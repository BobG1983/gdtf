//! The GAME host's command set — the one list, its registration walk, and the commands
//! themselves (GTW-942).
//!
//! Adding a command to this host is a file under [`read`] (or a future sibling family),
//! one line in [`set`], and a test. It moves no protocol version, adds no wire variant and
//! changes no courier, because a command is DATA carried inside two frozen envelope
//! variants rather than a variant of its own.
//!
//! ## Members (one concern per file, per module-layout)
//!
//! - [`set`] — [`GAME_COMMANDS`](set::GAME_COMMANDS), the ONE list, and the host name its
//!   catalogue reports.
//! - [`register`] — [`register_game_commands`], one walk of that slice.
//! - [`read`] — the read commands, one file each.
//! - `conformance` (feature `test-support`) — the two per-host conformance assertions, run
//!   over the real slice from inside the module that owns it.

#[cfg(feature = "test-support")]
pub(crate) mod conformance;
pub(crate) mod read;
pub(crate) mod register;
pub(crate) mod set;

// The two conformance entry points are `pub` in a `test-support` build only, and the ledger
// in `crate::test_support` is what makes them reachable. Unlike the SLICE, neither names a
// type private to this module, so exporting them drags nothing onto the public surface.
#[cfg(feature = "test-support")]
pub use conformance::{assert_game_command_set_is_conformant, game_command_names};
pub(in crate::dev::net_qa) use register::register_game_commands;
pub(in crate::dev::net_qa) use set::{GAME_COMMANDS, game_host_name};
