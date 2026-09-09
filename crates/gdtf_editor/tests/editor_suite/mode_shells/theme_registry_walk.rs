//! Every shipped theme def, asked for the file the theme family read it from.

use gdtf_assets::ContentSourcePaths;
use gdtf_battle_sim::level::UuidThemeRegistry;
use gdtf_content_families::ThemeDefsFamily;
use gdtf_editor::theme_source;

use crate::mode_shells::support::{advance_to_editing, editor_app};

#[test]
fn every_shipped_theme_names_the_file_it_was_read_from() {
    let mut app = editor_app();
    advance_to_editing(&mut app);

    let Some(registry) = app.world().get_resource::<UuidThemeRegistry>().cloned() else {
        unreachable!("the UuidThemeRegistry must be in the world once the editor is Editing")
    };
    assert!(
        registry.defs().next().is_some(),
        "the shipped registry must hold at least one theme, or the walk below passes on zero \
         iterations",
    );

    let Some(sources) = app
        .world()
        .get_resource::<ContentSourcePaths<ThemeDefsFamily>>()
    else {
        unreachable!("the theme family publishes its source paths beside the registry")
    };
    for (key, def) in registry.defs() {
        assert!(
            theme_source(Some(sources), *key).is_some(),
            "{} ({key:?}) is in the shipped registry, so the family must name the file it was \
             read from; without it a save after a load rebuilds a path from the display name",
            *def.display_name,
        );
    }
}
