//! Every shipped terrain def, opened in the Terrain form and projected back out again.

use bevy::asset::uuid::Uuid;
use gdtf_assets::ContentSourcePaths;
use gdtf_battle_sim::terrain::def::{TerrainDefRegistry, TerrainUuid};
use gdtf_content_families::TerrainDefsFamily;
use gdtf_editor::{TerrainDraft, draft_to_terrain_def, terrain_source};

use crate::mode_shells::support::{advance_to_editing, editor_app};

// The authored keys of the six survivors, one per kind the form can author.
const SURVIVORS: [(&str, &str); 6] = [
    ("bulkhead_wall", "00000000-0000-0000-0000-01840a910002"),
    ("door_ns", "00000000-0000-0000-0000-01840a91000b"),
    ("stair_ns_up", "00000000-0000-0000-0000-01840a91000d"),
    ("deck_slab", "00000000-0000-0000-0000-01840a910005"),
    ("barricade", "00000000-0000-0000-0000-01840a910001"),
    (
        "heavy_bolter_emplacement",
        "00000000-0000-0000-0000-01840a910011",
    ),
];

// Every survivor is in the registry, so a walk over it runs on more than zero rows.
fn assert_survivors_present(registry: &TerrainDefRegistry) {
    for (name, text) in SURVIVORS {
        let Ok(parsed) = Uuid::parse_str(text) else {
            unreachable!("{name}'s authored key must be hyphenated UUID text: {text}")
        };
        assert!(
            registry.def(&TerrainUuid::new(parsed)).is_some(),
            "the shipped registry must hold {name} ({text}), or the walk below passes on zero \
             iterations",
        );
    }
}

#[test]
fn every_shipped_def_names_the_file_it_was_read_from() {
    let mut app = editor_app();
    advance_to_editing(&mut app);

    let Some(registry) = app.world().get_resource::<TerrainDefRegistry>().cloned() else {
        unreachable!("the TerrainDefRegistry must be in the world once the editor is Editing")
    };
    assert_survivors_present(&registry);

    let Some(sources) = app
        .world()
        .get_resource::<ContentSourcePaths<TerrainDefsFamily>>()
    else {
        unreachable!("the terrain family publishes its source paths beside the registry")
    };
    for (key, def) in registry.defs() {
        assert!(
            terrain_source(Some(sources), *key).is_some(),
            "{} ({key:?}) is in the shipped registry, so the family must name the file it was \
             read from; without it a save after a load rebuilds a path from the display name",
            *def.display_name,
        );
    }
}

#[test]
fn every_shipped_def_reprojects_the_views_it_was_authored_with() {
    let mut app = editor_app();
    advance_to_editing(&mut app);

    let Some(registry) = app.world().get_resource::<TerrainDefRegistry>().cloned() else {
        unreachable!("the TerrainDefRegistry must be in the world once the editor is Editing")
    };
    assert_survivors_present(&registry);

    for (key, def) in registry.defs() {
        let mut draft = TerrainDraft::default();
        draft.load_from_def(def, None);
        let Ok(projected) = draft_to_terrain_def(&draft, *key) else {
            unreachable!(
                "{} ({key:?}) is a shipped def, so opening it in the form must project back",
                *def.display_name,
            )
        };
        assert_eq!(
            *projected.views, *def.views,
            "opening {} ({key:?}) in the Terrain form and saving it again must write the same \
             views it was authored with, row for row — projected {:?}, authored {:?}",
            *def.display_name, *projected.views, *def.views,
        );
    }
}
