//! The authored vertical-link shape — the [`OneWay`] directionality flag, the
//! an authored situation deserialises.

use bevy::prelude::Deref;
use serde::{Deserialize, Serialize};

use crate::metric::CellLevel;

/// `#[serde(transparent)]` lets an authored directionality parse as a bare boolean.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Deserialize, Serialize)]
#[serde(transparent)]
pub struct OneWay(bool);

impl OneWay {
            #[must_use]
    pub const fn new(one_way: bool) -> Self {
        Self(one_way)
    }

        #[must_use]
    pub const fn bidirectional() -> Self {
        Self(false)
    }

        #[must_use]
    pub const fn forward_only() -> Self {
        Self(true)
    }

        #[must_use]
    pub const fn is_one_way(self) -> bool {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub enum LinkKind {
        Stair {
                        one_way: OneWay,
    },
        Ladder {
                        one_way: OneWay,
    },
}

impl LinkKind {
        #[must_use]
    pub const fn stair() -> Self {
        Self::Stair {
            one_way: OneWay::bidirectional(),
        }
    }

        #[must_use]
    pub const fn ladder() -> Self {
        Self::Ladder {
            one_way: OneWay::bidirectional(),
        }
    }

            #[must_use]
    pub const fn is_one_way(self) -> OneWay {
        match self {
            Self::Stair { one_way } | Self::Ladder { one_way } => one_way,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct VerticalLink {
        pub from: CellLevel,
        pub to:   CellLevel,
        pub kind: LinkKind,
}

impl VerticalLink {
            #[must_use]
    pub const fn new(from: CellLevel, to: CellLevel, kind: LinkKind) -> Self {
        Self { from, to, kind }
    }
}
