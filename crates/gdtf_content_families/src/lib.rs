//! Content families that load authored RON into battle-sim registries.

mod armor;
mod attachments;
mod fields;
mod gangs;
/// Injury def and weighting load path.
pub mod injuries;
mod melee_weapons;
pub mod prefabs;
/// Sprite def load path and registry.
pub mod sprites;
mod terrain_defs;
mod theme_defs;
pub mod validate;
mod weapons;

pub use armor::ArmorFamily;
pub use attachments::AttachmentsFamily;
pub use fields::FieldsFamily;
pub use gangs::GangsFamily;
pub use melee_weapons::MeleeWeaponsFamily;
pub use prefabs::PrefabsFamily;
pub use sprites::SpriteDefsFamily;
pub use terrain_defs::TerrainDefsFamily;
pub use theme_defs::ThemeDefsFamily;
pub use weapons::WeaponsFamily;
