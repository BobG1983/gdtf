use bevy_derive::Deref;
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ShotName(String);

impl ShotName {
        #[must_use]
    pub const fn new(name: String) -> Self {
        Self(name)
    }
}
