//! Authored vertical-link shapes: stairs, ladders, and directionality.

use bevy::prelude::Deref;
use serde::{Deserialize, Serialize};

use crate::metric::CellLevel;

/// Whether a link is one-way (true) or bidirectional (false).
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Deserialize, Serialize)]
#[serde(transparent)]
pub struct OneWay(bool);

impl OneWay {
    #[must_use]
    pub const fn new(one_way: bool) -> Self {
        Self(one_way)
    }

    /// Bidirectional link.
    #[must_use]
    pub const fn bidirectional() -> Self {
        Self(false)
    }

    /// Forward-only link.
    #[must_use]
    pub const fn forward_only() -> Self {
        Self(true)
    }

    #[must_use]
    pub const fn is_one_way(self) -> bool {
        self.0
    }
}

/// Kind of vertical link.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub enum LinkKind {
    /// Stair link.
    Stair {
        /// Directionality.
        one_way: OneWay,
    },
    /// Ladder link.
    Ladder {
        /// Directionality.
        one_way: OneWay,
    },
}

impl LinkKind {
    /// Bidirectional stair.
    #[must_use]
    pub const fn stair() -> Self {
        Self::Stair {
            one_way: OneWay::bidirectional(),
        }
    }

    /// Bidirectional ladder.
    #[must_use]
    pub const fn ladder() -> Self {
        Self::Ladder {
            one_way: OneWay::bidirectional(),
        }
    }

    /// Directionality of this link.
    #[must_use]
    pub const fn is_one_way(self) -> OneWay {
        match self {
            Self::Stair { one_way } | Self::Ladder { one_way } => one_way,
        }
    }
}

/// One authored vertical connection between two cells.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct VerticalLink {
    /// Origin cell.
    pub from: CellLevel,
    /// Destination cell.
    pub to: CellLevel,
    /// Stair or ladder, and directionality.
    pub kind: LinkKind,
}

impl VerticalLink {
    #[must_use]
    pub const fn new(from: CellLevel, to: CellLevel, kind: LinkKind) -> Self {
        Self { from, to, kind }
    }
}
