//! GTW-663: the sprite-defs family's load coverage — the thin wrapper over
//! the generic per-family suite (`load_suite::suite`), plus the seed-truth
//! pin that keeps the GTW-663 seeded defs derived from the LIVE role table.
//!
//! The tier structure (`MinimalPlugins` no-op guard + gate pin, headless
//! real-asset folder resolve) is encoded ONCE in the shared suite; this file
//! binds it to [`SpriteDefsFamily`] with the authored member keys. The
//! `EXPECTED_MEMBERS` list is the FULL 20-name seed set (A2: every name
//! reachable as a `graphic_name` today — the whole `TileRole` vocabulary —
//! must parse through the real loader into the registry), and the
//! derivation-truth test cross-checks each seeded sheet/rect/anchor against
//! the shipped `tile_roles.spritedef.ron` + the presenter's sheet spec, so a
//! re-pointed role index cannot silently drift from its seeded def until
//! GTW-665 retires the table.

mod load_suite;

use std::path::Path;

use gdtf_assets::WORKSPACE_ASSETS_ROOT;
use gdtf_battle_presenter::{SheetRole, TileRole, TileRoles};
use gdtf_content_families::{
    SpriteDefsFamily,
    sprites::{SpriteDef, SpriteDefRegistry, SpriteName, SpriteSource},
};
use load_suite::suite::{self, FamilyLoadContract};

impl FamilyLoadContract for SpriteDefsFamily {
    /// The GTW-663 seed set: one def per `TileRole` key — EVERY name
    /// reachable as a terrain `graphic_name` today (the presenter resolves
    /// `graphic_name` through `TileRoles::index_for_key`, whose vocabulary is
    /// exactly these 20 role keys).
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

/// C2 seed-derivation truth (the gate's fidelity spot-check, made a standing
/// pin): every seeded `content/sprites/<role>.spritedef.ron` matches the LIVE
/// shipped role table — same sheet as the presenter's terrain sheet spec, the
/// rect computed from the table's CURRENT atlas index on that sheet's grid,
/// and the documented implicit center anchor (8, 8). Parses both artifacts
/// straight off disk (pure data↔data consistency — the loader path is tier
/// (b) above), so re-pointing a role index without re-deriving its seed
/// reddens THIS test, not a battle. Value-agnostic in spirit: no index
/// magnitude is pinned, only the AGREEMENT between the two authored files.
#[test]
fn seeded_defs_match_the_live_role_table_derivation() {
    let assets = Path::new(WORKSPACE_ASSETS_ROOT);
    let table = std::fs::read_to_string(assets.join("sprites/tile_roles.spritedef.ron"));
    assert!(table.is_ok(), "the shipped role table must read: {table:?}");
    let Ok(table) = table else { return };
    let roles = ron::de::from_str::<TileRoles>(&table);
    assert!(
        roles.is_ok(),
        "the shipped role table must parse: {roles:?}"
    );
    let Ok(roles) = roles else { return };

    let (columns, _rows) = SheetRole::Terrain.grid();
    let tile_px = SheetRole::Terrain.tile_px();
    for role in TileRole::ALL {
        let key = role.as_key();
        let path = assets.join(format!("content/sprites/{key}.spritedef.ron"));
        let seed = std::fs::read_to_string(&path);
        assert!(
            seed.is_ok(),
            "the seeded `{key}` def must exist at {path:?}"
        );
        let Ok(seed) = seed else { return };
        let def = ron::de::from_str::<SpriteDef>(&seed);
        assert!(def.is_ok(), "the seeded `{key}` def must parse: {def:?}");
        let Ok(def) = def else { return };

        // The derivation rule (C2): index -> (col, row) on the terrain
        // sheet's grid, in that sheet's own tile size.
        let index = u32::try_from(*role.index_in(&roles)).unwrap_or(u32::MAX);
        let expected_x = (index % columns) * tile_px;
        let expected_y = (index / columns) * tile_px;
        let source = def.source.clone();
        assert!(
            matches!(source, SpriteSource::Sheet { .. }),
            "the seeded `{key}` def must be a Sheet source, got {source:?}",
        );
        let SpriteSource::Sheet { sheet, rect } = source else {
            return;
        };
        assert_eq!(
            sheet.as_str(),
            SheetRole::Terrain.asset_path(),
            "the seeded `{key}` def must cut from the presenter's terrain sheet",
        );
        assert_eq!(
            (*rect.x, *rect.y, *rect.w, *rect.h),
            (expected_x, expected_y, tile_px, tile_px),
            "the seeded `{key}` rect must match the live table's index {index}",
        );
        // The documented CURRENT implicit anchor: sprite center == cell
        // center (the unit-quad draw at cell_to_world), i.e. half a tile.
        assert_eq!(
            (*def.anchor.x, *def.anchor.y),
            (tile_px / 2, tile_px / 2),
            "the seeded `{key}` anchor must document the implicit center anchor",
        );
    }
}
