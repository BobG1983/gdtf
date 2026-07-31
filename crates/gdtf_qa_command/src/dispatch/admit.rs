//! Resolving one incoming `Run` against a host's slice — without a wildcard and without a
//! string dispatch table.

use gdtf_qa_protocol::command::{
    CommandAvailability, CommandName, RefusalNote, RunOptions, UnavailableCode,
};

use crate::command::ErasedCommand;

/// A refusal — an availability that is known NOT to be
/// [`Available`](CommandAvailability::Available).
///
/// [`CommandAvailability`] has to carry the `Available` case because a catalogue row
/// publishes either answer. An ADMISSION refusal cannot be `Available`, so it gets its own
/// type: the route-time reply shaper takes one of these and is total, and a host's router
/// never has to write an arm for a case that cannot occur.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CommandRefusal {
    /// The machine-readable class of the refusal.
    code: UnavailableCode,
    /// The specific precondition, in words a caller can act on.
    note: RefusalNote,
}

impl CommandRefusal {
    /// Build a refusal from its class and its named precondition.
    #[must_use]
    pub const fn new(code: UnavailableCode, note: RefusalNote) -> Self {
        Self { code, note }
    }

    /// The refusal's class.
    #[must_use]
    pub const fn code(&self) -> UnavailableCode {
        self.code
    }

    /// The refusal's named precondition.
    #[must_use]
    pub const fn note(&self) -> &RefusalNote {
        &self.note
    }

    /// Split into the two parts a reply carries.
    #[must_use]
    pub fn into_parts(self) -> (UnavailableCode, RefusalNote) {
        (self.code, self.note)
    }

    /// The same refusal expressed as the catalogue's
    /// [`CommandAvailability`] — the form the "advertised and admitted agree" test
    /// compares against.
    #[must_use]
    pub fn to_availability(&self) -> CommandAvailability {
        CommandAvailability::Unavailable {
            code: self.code,
            note: self.note.clone(),
        }
    }
}

/// What the router decided about one incoming `Run`.
///
/// The [`Debug`] impl is hand-written: a derive would demand
/// `dyn ErasedCommand<F>: Debug`, and the trait deliberately declares no such bound —
/// every method on it returns a concrete type and nothing more.
pub enum Admission<'a, F> {
    /// Resolved and available — park the call for this command.
    Admit(&'a dyn ErasedCommand<F>),
    /// Resolved but not admissible right now.
    Unavailable(CommandRefusal),
    /// No command of that name on this host; every known name rides along so a typo
    /// self-corrects in one round trip.
    Unknown(Vec<CommandName>),
}

impl<F> core::fmt::Debug for Admission<'_, F> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Admit(command) => write!(f, "Admit({})", command.name().as_str()),
            Self::Unavailable(refusal) => write!(f, "Unavailable({refusal:?})"),
            Self::Unknown(known) => write!(f, "Unknown({known:?})"),
        }
    }
}

/// The refusal a call carrying an unbuilt rider gets.
///
/// [`RunOptions`] declares `await_ready` and `capture`; neither exists yet (GTW-941 stubs
/// them, C2 builds them). Running the command anyway and dropping the rider would answer a
/// question the caller did not ask, so the call is refused with the one code that says
/// "nothing about the app's state will make this work":
/// [`NotBuilt`](UnavailableCode::NotBuilt).
const fn rider_refusal(options: &RunOptions) -> Option<CommandRefusal> {
    if options.is_plain() {
        return None;
    }
    let note = match (options.await_ready.is_some(), options.capture.is_some()) {
        (true, true) => "this build has neither the await_ready nor the capture rider",
        (true, false) => "this build has no await_ready rider",
        _ => "this build has no capture rider",
    };
    Some(CommandRefusal::new(
        UnavailableCode::NotBuilt,
        RefusalNote::from_static(note),
    ))
}

/// Resolve `name` against `commands` and decide whether the call is admitted.
///
/// The name selects a TYPED ITEM from a list of typed items — it never selects a function
/// pointer from a hand-maintained pairing, and there is no `match` on it anywhere in this
/// crate. Availability comes from the command's own predicate, the same call
/// [`catalogue`](crate::catalogue::catalogue) makes, so the advertisement and the admission
/// cannot disagree.
///
/// The scan is linear over a slice of at most a few dozen entries, evaluated once per call
/// — a call an agent makes at human pace. It is not a hot path.
///
/// The order of the three checks is deliberate: an unknown NAME is reported as unknown even
/// when the call also carries an unbuilt rider, because the name is the more useful
/// correction; a known name with a rider is refused
/// [`NotBuilt`](UnavailableCode::NotBuilt) before its own predicate runs, since no state
/// the predicate could read would make the rider exist.
#[must_use]
pub fn admit<'a, F>(
    commands: &'a [&'a dyn ErasedCommand<F>],
    name: &CommandName,
    options: &RunOptions,
    facts: &F,
) -> Admission<'a, F> {
    let Some(command) = commands.iter().copied().find(|entry| entry.name() == *name) else {
        return Admission::Unknown(commands.iter().map(|entry| entry.name()).collect());
    };
    if let Some(refusal) = rider_refusal(options) {
        return Admission::Unavailable(refusal);
    }
    match command.availability(facts) {
        CommandAvailability::Available => Admission::Admit(command),
        CommandAvailability::Unavailable { code, note } => {
            Admission::Unavailable(CommandRefusal::new(code, note))
        }
    }
}
