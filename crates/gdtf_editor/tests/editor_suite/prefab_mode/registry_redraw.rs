//! C2/C3(b) — the painted PREVIEW keys on the sprite-def registry: a def
use std::collections::HashSet;

use bevy::prelude::*;
use gdtf_content_families::sprites::{SpriteAnchor, SpriteDefRegistry, SpritePx};

use super::harness::*;

fn sprite_ids(app: &mut App) -> HashSet<Entity> {
    let mut q = app.world_mut().query_filtered::<Entity, With<Sprite>>();
    q.iter(app.world()).collect()
}

#[test]
fn sprite_def_mutation_redraws_the_preview_tiles() {
    let mut app = editor_app();
    advance_to_editing(&mut app);
    for _ in 0..8 {
        app.update();
    }
    let before = sprite_ids(&mut app);
    assert!(
        !before.is_empty(),
        "precondition: the preview redraw must have spawned tile sprites",
    );

    app.update();
    assert_eq!(
        sprite_ids(&mut app),
        before,
        "a steady frame must not respawn the preview tiles (the change-driven guard)",
    );

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
