//! One load command per tab that loads, each scoped to the tab whose draft it fills.

mod armor;
mod attachment;
mod field;
mod gang;
mod injury;
mod melee_weapon;
mod prefab;
mod sprite;
mod terrain;
mod theme;
mod weapon;

pub(in crate::net_qa) use armor::EditorLoadArmor;
pub(in crate::net_qa) use attachment::EditorLoadAttachment;
pub(in crate::net_qa) use field::EditorLoadField;
pub(in crate::net_qa) use gang::EditorLoadGang;
pub(in crate::net_qa) use injury::EditorLoadInjury;
pub(in crate::net_qa) use melee_weapon::EditorLoadMeleeWeapon;
pub(in crate::net_qa) use prefab::EditorLoadPrefab;
pub(in crate::net_qa) use sprite::EditorLoadSprite;
pub(in crate::net_qa) use terrain::EditorLoadTerrain;
pub(in crate::net_qa) use theme::EditorLoadTheme;
pub(in crate::net_qa) use weapon::EditorLoadWeapon;
