//! GTW-663: the sprite-defs family's load coverage — the thin wrapper over
//! the generic per-family suite (`load_suite::suite`).
//!
//! The tier structure (`MinimalPlugins` no-op guard + gate pin, headless
//! real-asset folder resolve) is encoded ONCE in the shared suite; this file
//! binds it to [`SpriteDefsFamily`] with the authored member keys. The
//! `EXPECTED_MEMBERS` list is the FULL 20-name seed set (A2: every name
//! reachable as a `graphic_name` today — the whole `TileRole` vocabulary —
//! must parse through the real loader into the registry). The GTW-663
//! derivation-truth pin (seeded defs cross-checked against the live
//! `tile_roles.spritedef.ron`) retired WITH that table, per its own
//! retirement note: since GTW-665 the defs ARE what renders — there is no
//! second artifact to drift from.

mod load_suite;

use gdtf_content_families::{
    SpriteDefsFamily,
    sprites::{SpriteDefRegistry, SpriteName},
};
use load_suite::suite::{self, FamilyLoadContract};

impl FamilyLoadContract for SpriteDefsFamily {
    /// The GTW-663 seed set: one def per `TileRole` key — EVERY name
    /// reachable as a terrain `graphic_name` today (the presenter's GTW-665
    /// resolution looks each `graphic_name` up in the registry by NAME;
    /// the seeds cover the whole 20-key `TileRole` vocabulary).
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

/// AC (tier a) — the sprite-def loader registration + folder kick-off no-op
/// cleanly under `MinimalPlugins` (bevy-traps rule 1), with the GTW-629 rider
/// seeding the default registry.
#[test]
fn sprite_loader_no_ops_cleanly_without_asset_server() {
    suite::loader_no_ops_without_asset_server::<SpriteDefsFamily>();
}

/// AC (companion, tier a) — the Load→Intro transition GATES on the
/// [`SpriteDefRegistry`] (the GTW-663 gate clause: the sprite catalog is
/// verified loaded before `Load` exits, so the `graphic_name` integrity edge
/// — and, from GTW-665, the renderer — never reads an unresolved registry).
#[test]
fn load_does_not_leave_without_a_sprite_def_registry() {
    suite::load_gates_on_registry::<SpriteDefsFamily>();
}

/// AC (tier b) / A2 — the REAL `assets/content/sprites/` folder resolves into
/// a stem-keyed [`SpriteDefRegistry`] through the Load code path, and ALL 20
/// seeded members resolve (the registry holds them all).
#[test]
fn real_asset_resolves_sprite_def_registry_keyed_by_filename() {
    suite::real_asset_resolves_registry::<SpriteDefsFamily>();
}
