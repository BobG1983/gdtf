//! Asking for a delete, and how one settles.

use bevy::prelude::Resource;
use gdtf_assets::{ContentMemberKey, FindingFamily, ReferringRecord};

/// Ask to delete the record under `key` from the family `family` labels.
#[derive(Resource, Debug, Clone, PartialEq, Eq)]
pub struct DeleteRequest {
    family: FindingFamily,
    key:    ContentMemberKey,
}

impl DeleteRequest {
    /// Ask for one record to be deleted.
    #[must_use]
    pub const fn new(family: FindingFamily, key: ContentMemberKey) -> Self {
        Self { family, key }
    }

    /// The finding family label of the record to delete.
    #[must_use]
    pub const fn family(&self) -> &FindingFamily {
        &self.family
    }

    /// The key of the record to delete.
    #[must_use]
    pub const fn key(&self) -> &ContentMemberKey {
        &self.key
    }
}

/// Why a delete was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeleteRefusal {
    /// No delete entry names the requested family.
    NoEntry,
    /// The entry's registry holds no record under the requested key.
    NoRecord,
    /// Records still reference the requested record.
    InUse(Vec<ReferringRecord>),
}

/// How a delete settled.
#[derive(Resource, Debug, Clone, PartialEq, Eq)]
pub enum DeleteOutcome {
    /// The delete did not happen, for this reason.
    Refused(DeleteRefusal),
    /// The record is out of its registry and its file is gone.
    Removed,
}
