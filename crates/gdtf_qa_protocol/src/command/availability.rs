use std::borrow::Cow;

use bevy_derive::Deref;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CommandAvailability {
        Available,
        Unavailable {
                code: UnavailableCode,
                note: RefusalNote,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum UnavailableCode {
        WrongState,
        Replaying,
        MissingModel,
                                    NotBuilt,
}

impl UnavailableCode {
                    pub const ALL: [Self; 4] = [
        Self::WrongState,
        Self::Replaying,
        Self::MissingModel,
        Self::NotBuilt,
    ];
}

#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RefusalNote(Cow<'static, str>);

impl RefusalNote {
        #[must_use]
    pub const fn from_static(note: &'static str) -> Self {
        Self(Cow::Borrowed(note))
    }

        #[must_use]
    pub const fn from_owned(note: String) -> Self {
        Self(Cow::Owned(note))
    }

        #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
