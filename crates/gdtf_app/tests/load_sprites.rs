//! Load sprite defs into [`SpriteDefsFamily`].
//! Value-agnostic: registry presence only (no pinned stems).
mod load_suite;

use gdtf_content_families::{SpriteDefsFamily, sprites::SpriteDefRegistry};
use load_suite::suite::{self, FamilyLoadContract};

impl FamilyLoadContract for SpriteDefsFamily {
    fn is_empty(registry: &SpriteDefRegistry) -> bool {
        registry.is_empty()
    }
}

#[test]
fn sprite_loader_no_ops_cleanly_without_asset_server() {
    suite::loader_no_ops_without_asset_server::<SpriteDefsFamily>();
}

#[test]
fn load_does_not_leave_without_a_sprite_def_registry() {
    suite::load_gates_on_registry::<SpriteDefsFamily>();
}

#[test]
fn real_asset_resolves_sprite_def_registry_keyed_by_filename() {
    suite::real_asset_resolves_registry::<SpriteDefsFamily>();
}
