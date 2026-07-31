//! [`QaCommandSystems`] — the two ordered bands the command path runs in.

use bevy::prelude::*;

/// The two system sets the command path needs ordered, in the order they must run.
///
/// Both live in [`Update`], and [`register_command`](super::register_command) chains them
/// `Route` → `Claim` itself rather than leaving it to a host to remember: routing fills the
/// [`CommandInbox`](super::CommandInbox) and claiming empties it, so a claim that ran first
/// would leave every call a frame late. A host still ORDERS them against its own sets —
/// `Route.after(its input band)`, say — it just does not choose the schedule or the
/// relative order of these two.
///
/// Handlers are NOT a set here. A command's handler belongs to whatever band its work
/// belongs to — post-simulate for a sim read, the capture pump's band for a screenshot —
/// and only needs to run after [`Claim`](Self::Claim).
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum QaCommandSystems {
    /// The host's router: drain the socket inbox, answer `Catalogue`, admit each `Run`.
    Route,
    /// Every command's decode step — `claim_calls::<C>`, one per command.
    Claim,
}
