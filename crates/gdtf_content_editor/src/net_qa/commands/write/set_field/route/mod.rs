//! Which form answers a field write: one route per mode tab, each naming every field.

mod no_field;
mod with_field;

pub(super) use no_field::{prefab, theme};
pub(super) use with_field::{
    armor, attachment, field, gang, injury, melee_weapon, sprite, terrain, weapon,
};
