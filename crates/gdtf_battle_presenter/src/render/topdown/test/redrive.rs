use bevy::{
    MinimalPlugins,
    asset::{AssetApp, AssetEvent, AssetId, AssetPlugin, Assets, Handle},
    image::{Image, TextureAtlasLayout},
    platform::collections::HashMap,
    prelude::*,
};

use super::{
    super::{
        atlases::{SheetAtlas, SheetRole, TopDownAtlases},
        redrive::redrive_sheet_images_on_asset_event,
    },
    log_capture::capture_logs,
};
use crate::{Brightness, TerrainFogMaterial};

#[derive(Resource, Default)]
struct MaterialModifiedWitness {
        ids: Vec<AssetId<TerrainFogMaterial>>,
}

fn witness_material_modified(
    mut events: MessageReader<AssetEvent<TerrainFogMaterial>>,
    mut witness: ResMut<MaterialModifiedWitness>,
) {
    for event in events.read() {
        if let AssetEvent::Modified { id } = event {
            witness.ids.push(*id);
        }
    }
}

fn app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(AssetPlugin::default())
        .init_asset::<Image>()
        .init_asset::<TextureAtlasLayout>()
        .init_asset::<TerrainFogMaterial>()
        .init_resource::<MaterialModifiedWitness>()
        .add_systems(Update, witness_material_modified)
        .add_systems(Update, redrive_sheet_images_on_asset_event);
    app
}

fn add_image(app: &mut App) -> Handle<Image> {
    app.world_mut()
        .resource_mut::<Assets<Image>>()
        .add(Image::default())
}

fn add_material(app: &mut App, image: &Handle<Image>) -> Handle<TerrainFogMaterial> {
    app.world_mut()
        .resource_mut::<Assets<TerrainFogMaterial>>()
        .add(TerrainFogMaterial {
            image:        image.clone(),
            atlas_layout: None,
            atlas_index:  0,
            custom_size:  None,
            saturation:   1.0,
            brightness:   Brightness::FULL,
        })
}

fn insert_atlases(app: &mut App) -> (Handle<Image>, Handle<Image>) {
    let terrain = add_image(app);
    let characters = add_image(app);
    let layout = app
        .world_mut()
        .resource_mut::<Assets<TextureAtlasLayout>>()
        .add(TextureAtlasLayout::new_empty(UVec2::splat(16)));
    let mut sheets = HashMap::default();
    sheets.insert(
        SheetRole::Terrain,
        SheetAtlas {
            image:  terrain.clone(),
            layout: layout.clone(),
        },
    );
    sheets.insert(
        SheetRole::Characters,
        SheetAtlas {
            image: characters.clone(),
            layout,
        },
    );
    app.world_mut().insert_resource(TopDownAtlases { sheets });
    (terrain, characters)
}

fn inject_modified(app: &mut App, handle: &Handle<Image>) {
    app.world_mut()
        .write_message(AssetEvent::Modified { id: handle.id() });
}

fn settle(app: &mut App) {
    app.update();
    app.update();
}

fn modified_ids(app: &App) -> Vec<AssetId<TerrainFogMaterial>> {
    app.world()
        .resource::<MaterialModifiedWitness>()
        .ids
        .clone()
}

#[test]
fn terrain_image_modified_re_prepares_the_materials_sampling_it() {
    let mut app = app();
    let (terrain, characters) = insert_atlases(&mut app);
    let terrain_material = add_material(&mut app, &terrain);
    let other_material = add_material(&mut app, &characters);
    settle(&mut app);
    assert!(
        modified_ids(&app).is_empty(),
        "precondition: after settling, no material Modified may have been observed",
    );

    inject_modified(&mut app, &terrain);
    settle(&mut app);

    let ids = modified_ids(&app);
    assert!(
        ids.contains(&terrain_material.id()),
        "a Modified for the TERRAIN sheet image must re-prepare the material sampling it \
         (one get_mut deref → one AssetEvent::Modified rebuilding its snapshot bind group)",
    );
    assert!(
        !ids.contains(&other_material.id()),
        "a material sampling a DIFFERENT image must stay untouched — only the reloaded \
         image's materials re-prepare",
    );
}

#[test]
fn non_terrain_image_modified_leaves_other_materials_untouched() {
    let mut app = app();
    let (terrain, characters) = insert_atlases(&mut app);
    let terrain_material = add_material(&mut app, &terrain);
    let characters_material = add_material(&mut app, &characters);
    settle(&mut app);

    inject_modified(&mut app, &characters);
    settle(&mut app);

    let ids = modified_ids(&app);
    assert!(
        ids.contains(&characters_material.id()),
        "the material sampling the reloaded characters image must re-prepare",
    );
    assert!(
        !ids.contains(&terrain_material.id()),
        "a NON-terrain reload must leave the terrain-sampling material untouched",
    );
}

#[test]
fn unrelated_image_modified_touches_no_material() {
    let mut app = app();
    let (terrain, _characters) = insert_atlases(&mut app);
    let _terrain_material = add_material(&mut app, &terrain);
    let unrelated = add_image(&mut app);
    settle(&mut app);

    inject_modified(&mut app, &unrelated);
    settle(&mut app);

    assert!(
        modified_ids(&app).is_empty(),
        "a Modified for an id no material samples must not re-prepare anything",
    );
}

#[test]
fn redrive_does_not_panic_without_the_image_event_buffer() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_systems(Update, redrive_sheet_images_on_asset_event);
    app.update();
    app.update();
}

#[test]
fn non_terrain_sheet_reload_logs_an_info_line_naming_the_sheet() {
    use bevy::ecs::system::RunSystemOnce;

    let mut app = app();
    let (_terrain, characters) = insert_atlases(&mut app);
    inject_modified(&mut app, &characters);

    let captured = capture_logs(|| {
        let result = app
            .world_mut()
            .run_system_once(redrive_sheet_images_on_asset_event);
        assert!(result.is_ok(), "the redrive system must run cleanly");
    });

    assert!(
        captured
            .iter()
            .any(|line| line.contains("tileset hot-reload")
                && line.contains(SheetRole::Characters.asset_path())),
        "a non-terrain sheet reload must emit an info! line naming the reloaded sheet; \
         captured: {captured:?}",
    );
}
