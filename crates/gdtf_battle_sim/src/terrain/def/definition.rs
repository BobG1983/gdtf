use bevy::{prelude::Deref, reflect::TypePath};
use serde::{Deserialize, Serialize};

use super::{LosBlocking, TerrainPresenterKind, TerrainSimKind, TerrainTag, TerrainUuid};

/// terrain piece in tooling / authoring.
/// value). Private inner + derived [`Deref`]; `#[serde(transparent)]` parses a bare
#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize, TypePath)]
#[serde(transparent)]
pub struct TerrainDisplayName(String);

impl TerrainDisplayName {
        #[must_use]
    pub const fn new(name: String) -> Self {
        Self(name)
    }
}

/// `#[serde(transparent)]` round-trips it as the bare boolean wire form, so an authored
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(transparent)]
pub struct BlocksPathingOverride(bool);

impl BlocksPathingOverride {
            #[must_use]
    pub const fn new(over: bool) -> Self {
        Self(over)
    }
}

///   `#[serde(default)]` makes an omitted `tags` field parse as the EMPTY vec.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, TypePath)]
pub struct TerrainDef {
        pub key:            TerrainUuid,
        pub display_name:   TerrainDisplayName,
        pub sim_kind:       TerrainSimKind,
            pub presenter_kind: TerrainPresenterKind,
            /// (`#[serde(default)]`). Consumption is GTW-482, not this slice.
    #[serde(default)]
    pub tags:           Vec<TerrainTag>,
                /// `#[serde(default)]` (defaulting to `None`) so an omitted field is a piece with no death
    /// effect: the field is OPT-IN (the `tags` `#[serde(default)]` precedent), so EVERY existing
                            #[serde(default)]
    pub on_death:       Option<crate::effects::on_death::OnDeathEffect>,
                                            /// terrain-authoring "Step 3" code excursion this ticket kills). `#[serde(default)]` keeps
                                #[serde(default)]
    pub blocks_pathing: Option<BlocksPathingOverride>,
                                                /// `#[serde(default)]` keeps shipped `.ron` unchanged. Resolved by
                #[serde(default)]
    pub blocks_los:     Option<LosBlocking>,
}
