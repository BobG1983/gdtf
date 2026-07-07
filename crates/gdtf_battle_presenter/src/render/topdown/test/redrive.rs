//! Unit tests for the sheet-image hot-reload redrive surface (moved whole from the
//! old `bridge.rs` inline `mod test`; re-witnessed by GTW-666 — the redrive now
//! re-prepares the terrain MATERIALS sampling a reloaded image instead of poking
//! the `SpriteDefRegistry` into a full despawn+respawn redraw).

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

/// Probe resource: every [`AssetEvent::Modified`] id for [`TerrainFogMaterial`]
/// drained since the app was built — the GTW-666 re-prepare witness (a `get_mut`
/// deref on a material queues exactly one `Modified`, which is what rebuilds its
/// snapshot bind group against the fresh `GpuImage`).
#[derive(Resource, Default)]
struct MaterialModifiedWitness {
    /// The modified material ids, in drain order (cumulative).
    ids: Vec<AssetId<TerrainFogMaterial>>,
}

/// Downstream witness system: accumulate every `AssetEvent::Modified` for
/// [`TerrainFogMaterial`] (the store flushes its queued events end-of-frame, so
/// the probe observes a frame-N `get_mut` on frame N+1).
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

/// A headless app with the real sheet-image hot-reload wiring: `MinimalPlugins` +
/// `AssetPlugin`, the `Image` / `TextureAtlasLayout` / `TerrainFogMaterial` asset types
/// registered (so the stores and their `AssetEvent` message buffers exist), the real
/// `redrive_sheet_images_on_asset_event` in `Update`, and the cumulative material-Modified
/// witness alongside it.
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

/// Mint a fresh `Image` handle in the app's `Assets<Image>` and return it.
fn add_image(app: &mut App) -> Handle<Image> {
    app.world_mut()
        .resource_mut::<Assets<Image>>()
        .add(Image::default())
}

/// Add one [`TerrainFogMaterial`] sampling `image` and return its handle — the
/// drawn-tile stand-in whose bind group the redrive must (or must not) re-prepare.
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

/// Build a `TopDownAtlases` mapping `terrain` / `characters` to freshly-minted image
/// handles (a shared throwaway layout per sheet) and insert it as the resource, returning
/// the two image handles so the test can fire `Modified` for the right sheet.
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

/// Inject an `AssetEvent::Modified` for the given image id (standing in for the
/// file-watcher's reload signal).
fn inject_modified(app: &mut App, handle: &Handle<Image>) {
    app.world_mut()
        .write_message(AssetEvent::Modified { id: handle.id() });
}

/// Drive two settling updates so the material `Added` events drain and the witness's
/// baseline is clean (it accumulates `Modified` ONLY) before the event under test.
fn settle(app: &mut App) {
    app.update();
    app.update();
}

/// The accumulated modified-material ids.
fn modified_ids(app: &App) -> Vec<AssetId<TerrainFogMaterial>> {
    app.world()
        .resource::<MaterialModifiedWitness>()
        .ids
        .clone()
}

/// C4/C8(b), re-witnessed by GTW-666: a `Modified` for the TERRAIN sheet image re-prepares
/// EXACTLY the material sampling it — the redrive `get_mut`s that material (its snapshot
/// bind group must rebuild against the fresh `GpuImage`), queuing one `AssetEvent::Modified`
/// for it, and leaves a material sampling a DIFFERENT image untouched. Driven through the
/// REAL registered system via `app.update()`.
///
/// Pin-discriminating: dropping the material touch leaves the witness EMPTY; touching every
/// material regardless of image (e.g. dropping the id filter) also flags the
/// characters-sampling material and FAILS the second assert.
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

/// C10(e), re-witnessed by GTW-666: a `Modified` for a NON-terrain sheet image (characters)
/// is handled — logged, and any material sampling IT re-prepared — without touching the
/// terrain-sampling material.
///
/// Pin-discriminating: the modified set is NON-empty here, so a regression that touched
/// every material on ANY reload would flag the terrain material and FAIL.
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

/// C4/C8(b): a `Modified` for an UNRELATED image id (no material samples it, no sheet maps
/// it) touches NOTHING — zero material `Modified` events, and the store's resource tick
/// stays untouched (the redrive's `ResMut` binding never derefs when nothing matches).
#[test]
fn unrelated_image_modified_touches_no_material() {
    let mut app = app();
    let (terrain, _characters) = insert_atlases(&mut app);
    let _terrain_material = add_material(&mut app, &terrain);
    // An image handle that is NOT registered in TopDownAtlases and that no material
    // samples (a portrait node, a one-off texture, …).
    let unrelated = add_image(&mut app);
    settle(&mut app);

    inject_modified(&mut app, &unrelated);
    settle(&mut app);

    assert!(
        modified_ids(&app).is_empty(),
        "a Modified for an id no material samples must not re-prepare anything",
    );
}

/// C7: the redrive does NOT panic when the `AssetEvent<Image>` message buffer is ABSENT
/// (no image-asset stack) — the `Option<MessageReader<…>>` resolves to `None` and the
/// system no-ops. Built on bare `MinimalPlugins` (no `AssetPlugin`, no `init_asset`), so
/// there is no `Messages<AssetEvent<Image>>` buffer at all.
///
/// Pin-discriminating: a non-`Option` `MessageReader<AssetEvent<Image>>` param would trip
/// Bevy's param validation here and the update would fail.
#[test]
fn redrive_does_not_panic_without_the_image_event_buffer() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_systems(Update, redrive_sheet_images_on_asset_event);
    // No AssetPlugin / no init_asset::<Image>() ⇒ no Messages<AssetEvent<Image>> buffer.
    app.update();
    app.update();
}

/// C5/C8(b): the sheet-image hot-reload `info!` line FIRES on the real redrive path,
/// naming the reloaded sheet by its asset path — proven for a NON-terrain sheet
/// (characters) to pin the WIDENED (any-sheet) logging. Run via `run_system_once` on the
/// calling thread inside the scoped `tracing` subscriber so the thread-local capture sees
/// the emission (GTW-374 thread-local capture lesson).
///
/// Pin-discriminating: a redrive that only logged the terrain sheet would leave the
/// capture without the characters path and this assert fails.
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
