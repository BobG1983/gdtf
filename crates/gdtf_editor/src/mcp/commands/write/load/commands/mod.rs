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

pub(in crate::mcp) use armor::EditorLoadArmor;
pub(in crate::mcp) use attachment::EditorLoadAttachment;
pub(in crate::mcp) use field::EditorLoadField;
pub(in crate::mcp) use gang::EditorLoadGang;
pub(in crate::mcp) use injury::EditorLoadInjury;
pub(in crate::mcp) use melee_weapon::EditorLoadMeleeWeapon;
pub(in crate::mcp) use prefab::EditorLoadPrefab;
pub(in crate::mcp) use sprite::EditorLoadSprite;
pub(in crate::mcp) use terrain::EditorLoadTerrain;
pub(in crate::mcp) use theme::EditorLoadTheme;
pub(in crate::mcp) use weapon::EditorLoadWeapon;
