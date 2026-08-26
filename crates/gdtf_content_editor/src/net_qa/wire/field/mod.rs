//! Which single-value field a write names, with the value on its own variant.

mod draft_name;
mod editor_field;
mod value;

pub(in crate::net_qa) use draft_name::EditorDraftNameNet;
pub(in crate::net_qa) use editor_field::EditorFieldNet;
#[cfg(test)]
pub(in crate::net_qa) use value::FieldTurnsNet;
pub(in crate::net_qa) use value::{FieldDamageNet, FieldDurationNet};
