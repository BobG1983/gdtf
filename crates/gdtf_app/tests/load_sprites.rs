//! binds it to [`SpriteDefsFamily`] with the authored member keys. The
mod load_suite;

use gdtf_content_families::{
    SpriteDefsFamily,
    sprites::{SpriteDefRegistry, SpriteName},
};
use load_suite::suite::{self, FamilyLoadContract};

impl FamilyLoadContract for SpriteDefsFamily {
                    const EXPECTED_MEMBERS: &'static [&'static str] = &[
        "floor",
        "floor_alt_panel",
        "wall",
        "wall_ew",
        "cover",
        "emplacement",
        "emplacement_occupied",
        "slab",
        "rubble",
        "slab_destroyed",
        "door",
        "stair_up",
        "stair_down",
        "ladder",
        "door_ns",
        "door_ew",
        "stair_ns_up",
        "stair_ns_down",
        "stair_ew_up",
        "stair_ew_down",
    ];

    fn is_empty(registry: &SpriteDefRegistry) -> bool {
        registry.is_empty()
    }

    fn member_resolves(registry: &SpriteDefRegistry, label: &str) -> bool {
        registry.def(&SpriteName::new(label.to_owned())).is_some()
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
