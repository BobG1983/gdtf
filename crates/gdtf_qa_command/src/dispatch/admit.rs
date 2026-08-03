use gdtf_qa_protocol::command::{
    CommandAvailability, CommandName, RefusalNote, RunOptions, UnavailableCode,
};

use crate::command::ErasedCommand;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CommandRefusal {
        code: UnavailableCode,
        note: RefusalNote,
}

impl CommandRefusal {
        #[must_use]
    pub const fn new(code: UnavailableCode, note: RefusalNote) -> Self {
        Self { code, note }
    }

        #[must_use]
    pub const fn code(&self) -> UnavailableCode {
        self.code
    }

        #[must_use]
    pub const fn note(&self) -> &RefusalNote {
        &self.note
    }

        #[must_use]
    pub fn into_parts(self) -> (UnavailableCode, RefusalNote) {
        (self.code, self.note)
    }

                #[must_use]
    pub fn to_availability(&self) -> CommandAvailability {
        CommandAvailability::Unavailable {
            code: self.code,
            note: self.note.clone(),
        }
    }
}

pub enum Admission<'a, F> {
        Admit(&'a dyn ErasedCommand<F>),
        Unavailable(CommandRefusal),
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
