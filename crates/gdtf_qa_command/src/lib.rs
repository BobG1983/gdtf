//! Host-side QA command registration, admission, and Bevy dispatch.

/// Build a command catalogue from erased commands and host facts.
pub mod catalogue;
/// Typed command trait, erased form, and JSON schema helpers.
pub mod command;
/// Admit, claim, reply, and schedule command calls on a Bevy app.
pub mod dispatch;
/// Test-only fakes and assertions for host command sets.
pub mod test_support;
