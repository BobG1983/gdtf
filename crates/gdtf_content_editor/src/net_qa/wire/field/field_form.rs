//! The Field form's own fields, one arm per control its panels draw.

use serde::{Deserialize, Serialize};

use super::{
    draft_name::EditorDraftNameNet,
    value::{FieldDamageNet, FieldDurationNet},
};
use crate::net_qa::wire::attachment::DamageTypeNet;

/// One field of the Field draft, carrying the value it is set to.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(in crate::net_qa) enum FieldFormFieldNet {
    /// The draft's save stem.
    Name(EditorDraftNameNet),
    /// The draft's drain per tick.
    Damage(FieldDamageNet),
    /// The draft's damage channel.
    DamageType(DamageTypeNet),
    /// The draft's lifetime, where a zero turn count is refused rather than clamped.
    Duration(FieldDurationNet),
}
