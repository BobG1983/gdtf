//! GTW-666 C2/C3(b) — the painted PREVIEW keys on the sprite-def registry: a def
//! mutation re-runs the change-driven redraw (the preview tiles respawn against the
//! new registry), while a steady frame respawns nothing (the change-driven guard).

use std::collections::HashSet;

use bevy::prelude::*;
use gdtf_content_families::sprites::{SpriteAnchor, SpriteDefRegistry, SpritePx};

use super::harness::*;

/// The current preview-sprite entity id set (the harness spawns no other world
/// sprites — the `preview.rs` precedent — so the `Sprite` set IS the preview tiles).
fn sprite_ids(app: &mut App) -> HashSet<Entity> {
    let mut q = app.world_mut().query_filtered::<Entity, With<Sprite>>();
    q.iter(app.world()).collect()
}

/// A sprite-def registry mutation (a def EDIT — rect/anchor changed, path kept)
/// re-runs the preview redraw: the tile set respawns against the new registry,
/// where a steady frame leaves it untouched.
///
/// The dirty-set line under test: `redraw_preview_tiles`'s `sprites.is_changed()`
/// arm (`crates/gdtf_content_editor/src/preview/tiles/redraw.rs`). Deleting that
/// arm leaves the old ids in place after the mutation and this test FAILS.
#[test]
fn sprite_def_mutation_redraws_the_preview_tiles() {
    let mut app = editor_app();
    advance_to_editing(&mut app);
    // Let the seed + async registries resolve and the first redraw run.
    for _ in 0..8 {
        app.update();
    }
    let before = sprite_ids(&mut app);
    assert!(
        !before.is_empty(),
        "precondition: the preview redraw must have spawned tile sprites",
    );

    // STEADY-frame control: nothing changed, so the change-driven redraw must NOT
    // respawn — the id set is identical.
    app.update();
    assert_eq!(
        sprite_ids(&mut app),
        before,
        "a steady frame must not respawn the preview tiles (the change-driven guard)",
    );

    // The DEF EDIT: mutate one loaded def in place (anchor moved — rect/anchor
    // change with the path kept, exactly what a SPRITE-mode re-save produces when
    // the family redrive rebuilds the registry).
    {
        let mut registry = app.world_mut().resource_mut::<SpriteDefRegistry>();
        let edited = registry
            .defs()
            .next()
            .map(|(name, def)| (name.clone(), def.clone()));
        let Some((name, mut def)) = edited else {
            unreachable!("the editor Load pass resolved a non-empty sprite-def registry");
        };
        def.anchor = SpriteAnchor {
            x: SpritePx::new(0),
            y: SpritePx::new(0),
        };
        registry.insert(name, def);
    }
    app.update();

    let after = sprite_ids(&mut app);
    assert!(
        !after.is_empty(),
        "the redraw must respawn preview tiles after the def mutation",
    );
    assert!(
        after.is_disjoint(&before),
        "a sprite-def mutation must re-run the despawn-all redraw — every preview tile \
         respawned against the new registry (old ids gone)",
    );
}
