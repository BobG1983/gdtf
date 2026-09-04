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
mod theme;
mod value;
mod weapon;
mod weighting;

pub(in crate::mcp) use armor::ArmorFieldNet;
pub(in crate::mcp) use attachment::AttachmentFieldNet;
pub(in crate::mcp) use draft_name::EditorDraftNameNet;
pub(in crate::mcp) use editor_field::EditorFieldNet;
pub(in crate::mcp) use field_form::FieldFormFieldNet;
pub(in crate::mcp) use gang::GangFieldNet;
pub(in crate::mcp) use injury::InjuryFieldNet;
pub(in crate::mcp) use melee_weapon::MeleeWeaponFieldNet;
pub(in crate::mcp) use sprite::SpriteFieldNet;
pub(in crate::mcp) use terrain::TerrainFieldNet;
pub(in crate::mcp) use theme::ThemeFieldNet;
#[cfg(test)]
pub(in crate::mcp) use value::FieldTurnsNet;
pub(in crate::mcp) use value::{FieldDamageNet, FieldDurationNet};
pub(in crate::mcp) use weapon::WeaponFieldNet;
pub(in crate::mcp) use weighting::WeightingFieldNet;
