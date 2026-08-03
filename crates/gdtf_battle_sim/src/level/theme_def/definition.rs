use bevy::{prelude::Deref, reflect::TypePath};
use serde::{Deserialize, Serialize};

use super::ThemeUuid;
use crate::terrain::def::TerrainUuid;

/// value). Private inner + derived [`Deref`]; `#[serde(transparent)]` parses a bare RON
#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize, TypePath)]
#[serde(transparent)]
pub struct ThemeDisplayName(String);

impl ThemeDisplayName {
        #[must_use]
    pub const fn new(name: String) -> Self {
        Self(name)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, TypePath)]
pub struct UuidThemeDef {
        pub key:           ThemeUuid,
        pub display_name:  ThemeDisplayName,
                pub default_floor: TerrainUuid,
            pub terrain:       Vec<TerrainUuid>,
}
