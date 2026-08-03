use bevy_derive::Deref;
use serde::{Deserialize, Serialize};

#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CommandArgsJson(String);

impl CommandArgsJson {
        #[must_use]
    pub const fn new(json: String) -> Self {
        Self(json)
    }

        #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CommandReplyJson(String);

impl CommandReplyJson {
        #[must_use]
    pub const fn new(json: String) -> Self {
        Self(json)
    }

        #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ArgumentFault(String);

impl ArgumentFault {
        #[must_use]
    pub const fn new(detail: String) -> Self {
        Self(detail)
    }

        #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
