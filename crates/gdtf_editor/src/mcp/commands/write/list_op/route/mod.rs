//! Which form answers a list op: one route per mode tab, each naming every list.

mod no_list;
mod with_list;

pub(super) use no_list::{armor, prefab, theme};
pub(super) use with_list::{
    attachment, field, gang, injury, melee_weapon, sprite, terrain, weapon,
};
