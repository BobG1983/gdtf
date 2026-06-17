//! Pure status-panel format helpers (GTW-252, AC5): the per-vital line renderers live
//! in the `format` submodule. Tests live in the sibling `test` submodule (GTW-201).

mod format;
pub(in crate::scenes::running::game::battlescape::status_panel) use format::{
    NO_SELECTION, hp_label, identity_label, life_label, stance_label, tu_label, weapon_name_label,
};

#[cfg(test)]
mod test;
