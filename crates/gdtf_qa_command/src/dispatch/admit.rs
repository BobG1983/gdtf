//! Admit or refuse a command by name against the host command set.

use gdtf_qa_protocol::command::{CommandAvailability, CommandName, RefusalNote, UnavailableCode};

use crate::command::ErasedCommand;

/// Why a known command was refused.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CommandRefusal {
    code: UnavailableCode,
    note: RefusalNote,
}

impl CommandRefusal {
    /// Build a refusal from a code and note.
    #[must_use]
    pub const fn new(code: UnavailableCode, note: RefusalNote) -> Self {
        Self { code, note }
    }

    /// Refusal code.
    #[must_use]
    pub const fn code(&self) -> UnavailableCode {
        self.code
    }

    /// Human note.
    #[must_use]
    pub const fn note(&self) -> &RefusalNote {
        &self.note
    }

    /// Split into code and note.
    #[must_use]
    pub fn into_parts(self) -> (UnavailableCode, RefusalNote) {
        (self.code, self.note)
    }

    /// Convert to a protocol availability value.
    #[must_use]
    pub fn to_availability(&self) -> CommandAvailability {
        CommandAvailability::Unavailable {
            code: self.code,
            note: self.note.clone(),
        }
    }
}

/// Result of looking up a command by name.
pub enum Admission<'a, F> {
    /// Command is known and available.
    Admit(&'a dyn ErasedCommand<F>),
    /// Command is known but not available.
    Unavailable(CommandRefusal),
    /// Name is not in the set; payload is the known names.
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

/// Look up `name` in `commands` and check its availability against host facts.
#[must_use]
pub fn admit<'a, F>(
    commands: &'a [&'a dyn ErasedCommand<F>],
    name: &CommandName,
    facts: &F,
) -> Admission<'a, F> {
    let Some(command) = commands.iter().copied().find(|entry| entry.name() == *name) else {
        return Admission::Unknown(commands.iter().map(|entry| entry.name()).collect());
    };
    match command.availability(facts) {
        CommandAvailability::Available => Admission::Admit(command),
        CommandAvailability::Unavailable { code, note } => {
            Admission::Unavailable(CommandRefusal::new(code, note))
        }
    }
}
