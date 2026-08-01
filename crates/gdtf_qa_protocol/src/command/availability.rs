//! Whether a command can run right now — [`CommandAvailability`], [`UnavailableCode`],
//! [`RefusalNote`] (GTW-939).

use std::borrow::Cow;

use bevy_derive::Deref;
use serde::{Deserialize, Serialize};

/// Whether a command can run right now, and if not, why not.
///
/// No host computes this value yet — the admission path is GTW-941's (A3) work. It is
/// typed here so that path and the catalogue can share it: what a host advertises in its
/// [`CommandCatalogue`](crate::command::CommandCatalogue) and what it will accept on a
/// [`Run`](crate::message::QaRequest::Run) are meant to come from one predicate per
/// command rather than one central match that can drift from the list it advertises.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CommandAvailability {
    /// The command will be admitted right now.
    Available,
    /// The command exists but is not admissible in this state.
    Unavailable {
        /// The machine-readable class of the refusal.
        code: UnavailableCode,
        /// The specific precondition, in words a caller can act on.
        note: RefusalNote,
    },
}

/// The class of an availability refusal — deliberately host-neutral and tiny.
///
/// A code says which KIND of precondition failed, so a client can decide whether to wait,
/// to drive the app somewhere else, or to stop asking. The words that name the specific
/// precondition ride beside it in a [`RefusalNote`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum UnavailableCode {
    /// The app is not in a lifecycle state where this command means anything.
    WrongState,
    /// The screen has not finished showing what already happened.
    Replaying,
    /// The state this command reads or writes is not present in the world.
    MissingModel,
    /// The capability the call asked for is not built yet — a refusal from a host that
    /// has the command but not the machinery the call requested.
    ///
    /// The refusal a stubbed rider answers with (the
    /// [`RunOptions`](crate::command::RunOptions) riders are stubs until C2 builds them),
    /// and the refusal a per-mode editor write gives for a mode with no author path yet.
    /// Distinct from the other three: nothing about the app's state will make it
    /// available — waiting or navigating cannot help.
    NotBuilt,
}

impl UnavailableCode {
    /// Every refusal class, in declaration order.
    ///
    /// The list the round-trip suite walks to prove each code survives the wire; a new
    /// code that is not listed here fails that test.
    pub const ALL: [Self; 4] = [
        Self::WrongState,
        Self::Replaying,
        Self::MissingModel,
        Self::NotBuilt,
    ];
}

/// The words attached to a refusal — the precondition, named.
///
/// Private-inner newtype over `Cow<'static, str>` (no-bare-types), serde-transparent: a
/// host writes most of these as compile-time literals, while a decoding client owns its
/// copy.
#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RefusalNote(Cow<'static, str>);

impl RefusalNote {
    /// Build a refusal note from a compile-time literal.
    #[must_use]
    pub const fn from_static(note: &'static str) -> Self {
        Self(Cow::Borrowed(note))
    }

    /// Build a refusal note from a runtime string.
    #[must_use]
    pub const fn from_owned(note: String) -> Self {
        Self(Cow::Owned(note))
    }

    /// This note as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
