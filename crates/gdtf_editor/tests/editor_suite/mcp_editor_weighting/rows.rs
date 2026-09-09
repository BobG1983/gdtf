use gdtf_battle_sim::{armor::InjuryCategory, injuries::DamageContext, severity::Severity};
use serde::Deserialize;

use crate::mcp_shared::{mirror::ModeRow, save_fault::SaveFaultRow};

/// A client's own reading of an injury category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub(crate) enum CategoryRow {
    Head,
    Torso,
    Arm,
    Leg,
}

impl CategoryRow {
    /// Read the reply's category back as the sim's own.
    pub(crate) const fn to_category(self) -> InjuryCategory {
        match self {
            Self::Head => InjuryCategory::Head,
            Self::Torso => InjuryCategory::Torso,
            Self::Arm => InjuryCategory::Arm,
            Self::Leg => InjuryCategory::Leg,
        }
    }
}

/// A client's own reading of the damage source a table is authored for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub(crate) enum ContextRow {
    Ranged,
    Melee,
    Fall,
}

impl ContextRow {
    /// Read the reply's damage source back as the sim's own.
    pub(crate) const fn to_context(self) -> DamageContext {
        match self {
            Self::Ranged => DamageContext::Ranged,
            Self::Melee => DamageContext::Melee,
            Self::Fall => DamageContext::Fall,
        }
    }
}

/// A client's own reading of which bucket a row sits in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub(crate) enum BucketRow {
    Minor,
    Major,
    Critical,
}

impl BucketRow {
    /// The severity rank the bucket holds the rows for.
    pub(crate) const fn to_severity(self) -> Severity {
        match self {
            Self::Minor => Severity::Minor,
            Self::Major => Severity::Major,
            Self::Critical => Severity::Critical,
        }
    }
}

/// A client's own reading of one weighting row.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) struct WeightingRow {
    pub(crate) injury: String,
    pub(crate) weight: u32,
}

/// The reply body `editor.weighting` and `editor.select_weighting_table` both answer.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) struct WeightingTableRow {
    pub(crate) category: CategoryRow,
    pub(crate) context:  ContextRow,
    pub(crate) minor:    Vec<WeightingRow>,
    pub(crate) major:    Vec<WeightingRow>,
    pub(crate) critical: Vec<WeightingRow>,
}

impl WeightingTableRow {
    /// The rows the named bucket carries.
    pub(crate) fn bucket(&self, bucket: BucketRow) -> &[WeightingRow] {
        match bucket {
            BucketRow::Minor => &self.minor,
            BucketRow::Major => &self.major,
            BucketRow::Critical => &self.critical,
        }
    }
}

/// Which single-value field a weighting write named, under the form that owns it.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) enum FieldRow {
    Weighting(WeightingFieldRow),
}

/// A weighting row field a write named, read back off the draft.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) enum WeightingFieldRow {
    RowInjury {
        bucket: BucketRow,
        index:  usize,
        injury: String,
    },
    RowWeight {
        bucket: BucketRow,
        index:  usize,
        weight: u32,
    },
}

/// Which list a weighting write named.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub(crate) enum ListRow {
    WeightingBucket(BucketRow),
}

/// One member of the bucket a reply reads back.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) enum ListMemberRow {
    WeightingRow(WeightingRow),
}

/// `editor.set_field`'s reply body.
#[derive(Debug, Deserialize)]
pub(crate) struct SetFieldReplyRow {
    pub(crate) field: FieldRow,
}

/// `editor.list_op`'s reply body.
#[derive(Debug, Deserialize)]
pub(crate) struct ListOpReplyRow {
    pub(crate) list:    ListRow,
    pub(crate) members: Vec<ListMemberRow>,
}

/// What `editor.save_weighting` did. It never refuses, so no refusal arm is spelled here.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) enum SaveOutcomeRow {
    Wrote { path: String },
    Failed(SaveFaultRow),
}

/// `editor.save_weighting`'s reply body.
#[derive(Debug, Deserialize)]
pub(crate) struct SaveWeightingReplyRow {
    pub(crate) outcome: SaveOutcomeRow,
}

/// What the newest save under one mode did, as `editor.last_save` reports it.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) enum LastSaveOutcomeRow {
    Wrote { path: String },
    Failed(SaveFaultRow),
}

/// One row of `editor.last_save`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) struct LastSaveRow {
    pub(crate) mode:    ModeRow,
    pub(crate) outcome: LastSaveOutcomeRow,
}

/// `editor.last_save`'s reply body.
#[derive(Debug, Deserialize)]
pub(crate) struct LastSaveReplyRow {
    pub(crate) records: Vec<LastSaveRow>,
}
