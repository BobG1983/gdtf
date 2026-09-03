//! How a record delete settled, and every record that held it back, on the wire.

use bevy::prelude::Deref;
use gdtf_assets::ReferringRecord;
use serde::{Deserialize, Serialize};

use crate::delete::{DeleteOutcome, DeleteRefusal};

/// A finding family label a delete names, as the text its findings carry.
#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub(in crate::net_qa) struct DeleteFamilyNet(String);

impl DeleteFamilyNet {
    /// Wrap a finding family label.
    #[must_use]
    pub(in crate::net_qa) const fn new(family: String) -> Self {
        Self(family)
    }
}

/// The key of the record a delete names, as its registry recorded it.
#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub(in crate::net_qa) struct DeleteKeyNet(String);

impl DeleteKeyNet {
    /// Wrap a content member key.
    #[must_use]
    pub(in crate::net_qa) const fn new(key: String) -> Self {
        Self(key)
    }
}

/// The field one referring record holds the reference in.
#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub(in crate::net_qa) struct ReferenceFieldNet(String);

/// One record that still names the record a delete asked for.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::net_qa) struct ReferringRecordNet {
    /// The family the referring record belongs to.
    family: DeleteFamilyNet,
    /// The referring record's own key.
    key:    DeleteKeyNet,
    /// The field the reference sits in.
    field:  ReferenceFieldNet,
}

impl ReferringRecordNet {
    /// Mirror one referring record the in-use check answered with.
    #[must_use]
    pub(in crate::net_qa) fn from_record(record: &ReferringRecord) -> Self {
        Self {
            family: DeleteFamilyNet::new((*record.family).clone()),
            key:    DeleteKeyNet::new((*record.key).clone()),
            field:  ReferenceFieldNet((*record.field).clone()),
        }
    }
}

/// Why a delete was refused, mirroring the editor's own reasons.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(in crate::net_qa) enum DeleteRefusalNet {
    /// No delete entry names the requested family.
    NoEntry,
    /// The entry's registry holds no record under the requested key.
    NoRecord,
    /// Records still reference the requested record.
    InUse(Vec<ReferringRecordNet>),
}

/// How a delete settled, mirroring the editor's own outcome.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(in crate::net_qa) enum DeleteOutcomeNet {
    /// The delete did not happen, for this reason.
    Refused(DeleteRefusalNet),
    /// The record is out of its registry and its file is gone.
    Removed,
}

impl DeleteOutcomeNet {
    /// Mirror the outcome the delete driver settled on, with no wildcard arm.
    #[must_use]
    pub(in crate::net_qa) fn from_outcome(outcome: &DeleteOutcome) -> Self {
        match outcome {
            DeleteOutcome::Removed => Self::Removed,
            DeleteOutcome::Refused(DeleteRefusal::NoEntry) => {
                Self::Refused(DeleteRefusalNet::NoEntry)
            }
            DeleteOutcome::Refused(DeleteRefusal::NoRecord) => {
                Self::Refused(DeleteRefusalNet::NoRecord)
            }
            DeleteOutcome::Refused(DeleteRefusal::InUse(records)) => {
                Self::Refused(DeleteRefusalNet::InUse(
                    records
                        .iter()
                        .map(ReferringRecordNet::from_record)
                        .collect(),
                ))
            }
        }
    }
}
