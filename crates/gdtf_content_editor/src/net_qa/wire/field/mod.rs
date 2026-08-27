//! Which single-value field a write names, one enum per form and the value on its own variant.

mod armor;
mod attachment;
mod draft_name;
mod editor_field;
mod field_form;
mod gang;
mod injury;
mod melee_weapon;
mod sprite;
mod terrain;
mod value;
mod weapon;
mod weighting;

pub(in crate::net_qa) use armor::ArmorFieldNet;
pub(in crate::net_qa) use attachment::AttachmentFieldNet;
pub(in crate::net_qa) use draft_name::EditorDraftNameNet;
pub(in crate::net_qa) use editor_field::EditorFieldNet;
pub(in crate::net_qa) use field_form::FieldFormFieldNet;
pub(in crate::net_qa) use gang::GangFieldNet;
pub(in crate::net_qa) use injury::InjuryFieldNet;
pub(in crate::net_qa) use melee_weapon::MeleeWeaponFieldNet;
pub(in crate::net_qa) use sprite::SpriteFieldNet;
pub(in crate::net_qa) use terrain::TerrainFieldNet;
#[cfg(test)]
pub(in crate::net_qa) use value::FieldTurnsNet;
pub(in crate::net_qa) use value::{FieldDamageNet, FieldDurationNet};
pub(in crate::net_qa) use weapon::WeaponFieldNet;
pub(in crate::net_qa) use weighting::WeightingFieldNet;
