mod table;

#[cfg(test)]
mod test;

pub(crate) use table::register_keybinds_hot_ron;
pub use table::{BoundKey, Keybinds};
