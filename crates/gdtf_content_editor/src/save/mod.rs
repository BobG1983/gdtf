//! The WHOLE module is `#[cfg(debug_assertions)]`-gated at its `mod` site (the GTW-429 gang-save
mod project;
mod types;

#[cfg(test)]
mod tests;

pub use project::{
    editor_map_to_prefab, prefab_save_path, prefab_save_path_in, sanitize_name, serialize_prefab,
};
#[cfg(debug_assertions)]
pub use project::{write_prefab, write_prefab_in};
pub use types::SavePrefabError;
