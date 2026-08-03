use bevy_derive::Deref;
use serde::{Deserialize, Serialize};

#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ArgSchemaJson(String);

impl ArgSchemaJson {
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
pub struct ReplySchemaJson(String);

impl ReplySchemaJson {
        #[must_use]
    pub const fn new(json: String) -> Self {
        Self(json)
    }

        #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
