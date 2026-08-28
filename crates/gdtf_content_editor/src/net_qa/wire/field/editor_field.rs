//! One field of a draft on the wire, named by the form it belongs to.

use serde::{Deserialize, Serialize};

use super::{
    armor::ArmorFieldNet, attachment::AttachmentFieldNet, field_form::FieldFormFieldNet,
    gang::GangFieldNet, injury::InjuryFieldNet, melee_weapon::MeleeWeaponFieldNet,
    sprite::SpriteFieldNet, terrain::TerrainFieldNet, theme::ThemeFieldNet, weapon::WeaponFieldNet,
    weighting::WeightingFieldNet,
};

/// One field of the active mode's draft, under the form that owns it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(in crate::net_qa) enum EditorFieldNet {
    /// A field of the Terrain form.
    Terrain(TerrainFieldNet),
    /// A field of the Armor form.
    Armor(ArmorFieldNet),
    /// A field of the Sprite form.
    Sprite(SpriteFieldNet),
    /// A field of the Attachment form.
    Attachment(AttachmentFieldNet),
    /// A field of the Injury def form.
    Injury(InjuryFieldNet),
    /// A field of the Injury tab's weighting sub-tab.
    Weighting(WeightingFieldNet),
    /// A field of the Melee Weapon form.
    MeleeWeapon(MeleeWeaponFieldNet),
    /// A field of the Gang form.
    Gang(GangFieldNet),
    /// A field of the Weapon form.
    Weapon(WeaponFieldNet),
    /// A field of the Field form.
    Field(FieldFormFieldNet),
    /// A field of the Theme form.
    Theme(ThemeFieldNet),
}
