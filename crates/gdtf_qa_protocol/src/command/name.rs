use std::borrow::Cow;

use bevy_derive::Deref;
use serde::{Deserialize, Serialize};

#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CommandName(Cow<'static, str>);

impl CommandName {
            #[must_use]
    pub const fn from_static(name: &'static str) -> Self {
        Self(Cow::Borrowed(name))
    }

        #[must_use]
    pub const fn from_owned(name: String) -> Self {
        Self(Cow::Owned(name))
    }

        #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CommandSummary(Cow<'static, str>);

impl CommandSummary {
            #[must_use]
    pub const fn from_static(text: &'static str) -> Self {
        Self(Cow::Borrowed(text))
    }

        #[must_use]
    pub const fn from_owned(text: String) -> Self {
        Self(Cow::Owned(text))
    }

        #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
