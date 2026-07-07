//! Unit tests for the sheet-image hot-reload redrive surface (moved whole from the
//! old `bridge.rs` inline `mod test`).

use std::{cell::RefCell, sync::OnceLock};

use bevy::{
    MinimalPlugins,
    asset::{AssetApp, AssetEvent, AssetPlugin, Assets, Handle},
    image::{Image, TextureAtlasLayout},
    log::{
        tracing::{
            Event, Subscriber,
            callsite::rebuild_interest_cache,
            field::{Field, Visit},
        },
        tracing_subscriber::{Layer, layer::Context, prelude::*, registry::Registry},
    },
    platform::collections::HashMap,
    prelude::*,
};
use gdtf_content_families::sprites::SpriteDefRegistry;

use super::super::{
    atlases::{SheetAtlas, SheetRole, TopDownAtlases},
    redrive::redrive_sheet_images_on_asset_event,
};

thread_local! {
    /// The buffer the process-global [`CaptureLayer`] appends captured event
    /// messages to FOR THE CURRENT THREAD — `Some(..)` only while a
    /// [`capture_logs`] body runs here, so a concurrent non-capturing test's
    /// events are dropped rather than bleeding into this capture.
    static CAPTURE_BUFFER: RefCell<Option<Vec<String>>> = const { RefCell::new(None) };
}

/// Ensures the process-global [`CaptureLayer`] subscriber is installed EXACTLY
/// ONCE for this test binary (the GTW-494 determinism recipe, applied here by
/// GTW-455): a scoped `with_default` capture races `tracing-core`'s
/// process-global per-callsite `Interest` cache under parallel test threads —
/// a concurrent thread's first-time emission can rebuild the cache while no
/// always-interested dispatcher is live and poison the captured callsite
/// `never`, so the `info!` short-circuits and the capture comes back empty.
/// One global always-interested default (plus a post-install
/// [`rebuild_interest_cache`] to heal callsites registered while the default
/// was still `NoSubscriber`) closes both windows — no lock, sleep, or retry.
static GLOBAL_CAPTURE: OnceLock<()> = OnceLock::new();

/// Install the process-global [`CaptureLayer`] if not already installed, then
/// re-evaluate cached callsite interest against it (see [`GLOBAL_CAPTURE`]).
fn install_global_capture() {
    GLOBAL_CAPTURE.get_or_init(|| {
        let subscriber = Registry::default().with(CaptureLayer);
        // First (and only) global default for this test process — a second call
        // would `Err`, which the `OnceLock` already prevents; no competing global
        // is installed in this binary (the harness adds no `LogPlugin`).
        let _ = bevy::log::tracing::subscriber::set_global_default(subscriber);
        rebuild_interest_cache();
    });
}

/// A `tracing` layer that records each event's `message` field into the CURRENT
/// THREAD's [`CAPTURE_BUFFER`] — the minimal capture needed to prove the `info!`
/// hot-reload line fired. Mirrors the GTW-494 shared recipe in `gdtf_app`'s
/// `hot_reload_test_support` (no shared util is reachable here).
struct CaptureLayer;

/// Pulls the `message` field's debug rendering out of a `tracing` event.
struct MessageVisitor {
    /// The captured message text, if a `message` field was visited.
    message: Option<String>,
}

impl Visit for MessageVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.message = Some(format!("{value:?}"));
        }
    }
}

impl<S: Subscriber> Layer<S> for CaptureLayer {
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let mut visitor = MessageVisitor { message: None };
        event.record(&mut visitor);
        let Some(message) = visitor.message else {
            return;
        };
        CAPTURE_BUFFER.with(|buffer| {
            if let Some(messages) = buffer.borrow_mut().as_mut() {
                messages.push(message);
            }
        });
    }
}

/// Run `body` with this thread's [`CAPTURE_BUFFER`] armed, returning every event
/// message the process-global [`CaptureLayer`] captured on this thread while it
/// ran (in emission order).
///
/// The capture is per-thread, so concurrent tests never see each other's events;
/// the redrive must run SYNCHRONOUSLY on this thread (the call site uses
/// `run_system_once`) so its `info!` lands here. Determinism rationale: see
/// [`GLOBAL_CAPTURE`] (GTW-455).
fn capture_logs(body: impl FnOnce()) -> Vec<String> {
    install_global_capture();
    let prior = CAPTURE_BUFFER.with(|buffer| buffer.borrow_mut().replace(Vec::new()));
    body();
    let captured =
        CAPTURE_BUFFER.with(|buffer| std::mem::replace(&mut *buffer.borrow_mut(), prior));
    captured.unwrap_or_default()
}

/// Probe resource: the value of `SpriteDefRegistry::is_changed()` observed by a downstream
/// system the LAST time it ran. The image-redrive test reads this to prove the terrain
/// redraw trigger (`set_changed()`) fired — a downstream `DetectChanges` witness, exactly
/// the C8 "assert the redraw signal becomes `is_changed` via a downstream system" shape
/// (GTW-665 re-anchored the signal from the retired `TileRoles` onto the registry).
#[derive(Resource, Default)]
struct DefsChangedWitness {
    /// Whether the `SpriteDefRegistry` was `is_changed()` when the witness system last ran.
    changed: bool,
}

/// Downstream witness system: records whether the `SpriteDefRegistry` is currently
/// `is_changed()`.
///
/// Ordered `.after(redrive_sheet_images_on_asset_event)` so a `set_changed()` the redrive
/// performs THIS frame is visible to it (change ticks compare against this system's own
/// last-run tick). Overwrites the witness each frame (no latching) so the read after the
/// event update reflects only that frame.
fn witness_defs_changed(defs: Res<SpriteDefRegistry>, mut witness: ResMut<DefsChangedWitness>) {
    witness.changed = defs.is_changed();
}

/// A headless app with the real sheet-image hot-reload wiring: `MinimalPlugins` +
/// `AssetPlugin`, the `Image` + `TextureAtlasLayout` asset types registered (so
/// `Assets<Image>` and the `AssetEvent<Image>` message buffer exist), the real
/// `redrive_sheet_images_on_asset_event` in `Update`, and the downstream change witness
/// ordered after it.
fn app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(AssetPlugin::default())
        .init_asset::<Image>()
        .init_asset::<TextureAtlasLayout>()
        .init_resource::<DefsChangedWitness>()
        .add_systems(
            Update,
            witness_defs_changed.after(redrive_sheet_images_on_asset_event),
        )
        .add_systems(Update, redrive_sheet_images_on_asset_event);
    app
}

/// Mint a fresh `Image` handle in the app's `Assets<Image>` and return it.
fn add_image(app: &mut App) -> Handle<Image> {
    app.world_mut()
        .resource_mut::<Assets<Image>>()
        .add(Image::default())
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

/// Drive two settling updates so the witness's last-run tick advances PAST the
/// `SpriteDefRegistry` / `TopDownAtlases` insert, leaving the witness reading `false` (nothing
/// changed) before the event under test — so a `true` afterwards is the redrive's poke.
fn settle(app: &mut App) {
    app.update();
    app.update();
}

/// C4/C8(b): a `Modified` for the TERRAIN sheet image triggers the terrain redraw — the
/// redrive calls `set_changed()` on the `SpriteDefRegistry`, which the downstream witness sees as
/// `is_changed()`. Driven through the REAL registered system via `app.update()`.
///
/// Pin-discriminating: dropping the terrain `set_changed()` leaves the witness `false`;
/// a wrong id map would not fire for the terrain id.
#[test]
fn terrain_image_modified_triggers_the_terrain_redraw() {
    let mut app = app();
    let (terrain, _characters) = insert_atlases(&mut app);
    app.world_mut()
        .insert_resource(SpriteDefRegistry::default());
    settle(&mut app);
    assert!(
        !app.world().resource::<DefsChangedWitness>().changed,
        "precondition: after settling, the SpriteDefRegistry must NOT be is_changed",
    );

    inject_modified(&mut app, &terrain);
    app.update();

    assert!(
        app.world().resource::<DefsChangedWitness>().changed,
        "a Modified for the TERRAIN sheet image must set_changed() the SpriteDefRegistry \
         (forcing the terrain re-render)",
    );
}

/// C10(e): a `Modified` for a LOADED NON-terrain sheet image (characters) is handled but
/// does NOT trigger the terrain redraw — the redrive logs the reload yet leaves the `SpriteDefRegistry`
/// untouched, because only the terrain sheet draws through the snapshot bind group. Driven
/// through the REAL registered system via `app.update()`, with the downstream witness
/// ordered `.after` it (mirrors `terrain_image_modified_triggers_the_terrain_redraw`).
///
/// Pin-discriminating: the `reloaded` set is NON-empty here (it contains `Characters`), so a
/// regression that poked terrain on ANY non-empty reload (e.g. `!reloaded.is_empty()` instead
/// of `reloaded.contains(&SheetRole::Terrain)`) would flip the witness `true` and FAIL this
/// assert — whereas the unrelated-id test below has an EMPTY `reloaded` and cannot catch it.
#[test]
fn non_terrain_sheet_image_modified_does_not_trigger_the_terrain_redraw() {
    let mut app = app();
    let (_terrain, characters) = insert_atlases(&mut app);
    app.world_mut()
        .insert_resource(SpriteDefRegistry::default());
    settle(&mut app);
    assert!(
        !app.world().resource::<DefsChangedWitness>().changed,
        "precondition: after settling, the SpriteDefRegistry must NOT be is_changed",
    );

    inject_modified(&mut app, &characters);
    app.update();

    assert!(
        !app.world().resource::<DefsChangedWitness>().changed,
        "a Modified for a LOADED NON-terrain sheet (characters) must be handled WITHOUT \
         marking the SpriteDefRegistry changed — only the terrain sheet pokes the terrain redraw",
    );
}

/// C4/C8(b): a `Modified` for a NON-sheet (unrelated) image id does NOT trigger the
/// terrain redraw — the id maps to no sheet, so the `SpriteDefRegistry` stays unchanged.
///
/// Pin-discriminating: a redrive that poked on ANY image event (not just a loaded sheet's)
/// would flip the witness `true` here.
#[test]
fn unrelated_image_modified_does_not_trigger_the_terrain_redraw() {
    let mut app = app();
    let (_terrain, _characters) = insert_atlases(&mut app);
    app.world_mut()
        .insert_resource(SpriteDefRegistry::default());
    // An image handle that is NOT registered in TopDownAtlases (a portrait node, a one-off
    // texture, …).
    let unrelated = add_image(&mut app);
    settle(&mut app);

    inject_modified(&mut app, &unrelated);
    app.update();

    assert!(
        !app.world().resource::<DefsChangedWitness>().changed,
        "a Modified for an id that is NOT a loaded sheet must NOT touch the SpriteDefRegistry",
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
    app.world_mut()
        .insert_resource(SpriteDefRegistry::default());
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
