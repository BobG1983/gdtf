//! GTW-549 loader tests — the attachment hot-reload rebuild (registry re-keyed from the
//! folder's members on a `Modified` event) and its `info!` log line.
//!
//! These drive the REAL loader code path: both tests run the actual
//! [`redrive_attachments_on_asset_event`] system inside a `MinimalPlugins` +
//! `AssetPlugin` app (the melee-weapons test-harness shape). Per the loader-tests rule
//! they assert STRUCTURE (key-resolves / rebuild-happens / log-emitted), never specific
//! shipped tunable magnitudes.

use bevy::{
    MinimalPlugins,
    asset::{AssetEvent, AssetPlugin, AssetServer, Assets, Handle, LoadedFolder},
    ecs::system::RunSystemOnce,
    prelude::*,
};
use gdtf_assets::{RonAsset, RonAssetAppExt};
use gdtf_battle_sim::weapon::{AttachmentName, AttachmentRegistry, AttachmentSpec};

use super::redrive_attachments_on_asset_event;
use crate::states::load::{
    resources::ActiveAttachmentsFolderHandle,
    systems::resolve::hot_reload_test_support::capture_logs,
};

/// An attachment spec with the given display name — parsed from inline RON so the test
/// does not hand-assemble the effect list. Returns `None` (assert-fail) on a parse error
/// rather than a denied `unwrap`. Proves the attachment spec parses (folder-load path).
fn attachment_spec(display_name: &str) -> Option<AttachmentSpec> {
    // GTW-554: `slot:` is REQUIRED on every item (the mount point the fit gate reads).
    let ron = format!("(display_name: \"{display_name}\", slot: Sight, effects: [Aim(0.4)])");
    let parsed = ron::de::from_str::<AttachmentSpec>(&ron);
    assert!(
        parsed.is_ok(),
        "attachment fixture must parse: {:?}",
        parsed.as_ref().err()
    );
    parsed.ok()
}

/// A headless app with the real attachment hot-reload wiring.
fn app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(AssetPlugin::default())
        .init_ron_asset_with_extensions::<AttachmentSpec>(vec!["attachment.ron"])
        .add_systems(Update, redrive_attachments_on_asset_event);
    app
}

/// Register a member attachment asset at `path` carrying `spec`, returning its typed
/// handle (the melee `add_member` mirror).
fn add_member(
    app: &mut App,
    path: &'static str,
    spec: AttachmentSpec,
) -> Handle<RonAsset<AttachmentSpec>> {
    let handle = app
        .world()
        .resource::<AssetServer>()
        .load::<RonAsset<AttachmentSpec>>(path);
    let inserted = app
        .world_mut()
        .resource_mut::<Assets<RonAsset<AttachmentSpec>>>()
        .insert(handle.id(), RonAsset::new(spec));
    assert!(inserted.is_ok(), "member spec insert must succeed");
    handle
}

/// Build a `LoadedFolder` over the given member handles, add it, return its handle.
fn add_folder(app: &mut App, members: &[Handle<RonAsset<AttachmentSpec>>]) -> Handle<LoadedFolder> {
    let folder = LoadedFolder {
        handles: members.iter().map(|h| h.clone().untyped()).collect(),
    };
    app.world_mut()
        .resource_mut::<Assets<LoadedFolder>>()
        .add(folder)
}

/// A `Modified` for a member `*.attachment.ron` REBUILDS the `AttachmentRegistry` from the
/// folder's members — keyed by file stem (minus the `.attachment` infix) — reflecting the
/// edited spec. Pin-discriminating: dropping the rebuild leaves the OLD display name;
/// mis-keying drops the `scoped_sight` entry. Asserts the KEY + rebuild mechanism, not a
/// magnitude.
#[test]
fn modified_member_rebuilds_attachment_registry() {
    let mut app = app();
    let Some(original) = attachment_spec("Scoped Sight") else {
        return;
    };
    let member = add_member(
        &mut app,
        "content/attachments/scoped_sight.attachment.ron",
        original,
    );
    let folder = add_folder(&mut app, std::slice::from_ref(&member));
    app.world_mut()
        .insert_resource(ActiveAttachmentsFolderHandle::new(folder));
    app.world_mut()
        .insert_resource(AttachmentRegistry::default());
    app.update();

    let Some(edited) = attachment_spec("Long Scope") else {
        return;
    };
    if let Some(mut asset) = app
        .world_mut()
        .resource_mut::<Assets<RonAsset<AttachmentSpec>>>()
        .get_mut(&member)
    {
        **asset = edited;
    }
    app.world_mut()
        .write_message(AssetEvent::Modified { id: member.id() });
    app.update();

    let key = AttachmentName::new("scoped_sight".to_owned());
    let name = app
        .world()
        .get_resource::<AttachmentRegistry>()
        .and_then(|r| r.spec(&key).map(|s| (*s.display_name).clone()));
    assert_eq!(
        name,
        Some("Long Scope".to_owned()),
        "the hot-reload must rebuild the registry, keyed by stem, with the edited spec",
    );
}

/// A hot-reload of an attachment member fires the `info!` line naming what reloaded. Run
/// via `run_system_once` so the thread-local `tracing` capture sees the emission.
/// Pin-discriminating: removing the `info!` leaves the capture empty.
#[test]
fn attachment_hot_reload_logs_an_info_line() {
    let mut app = app();
    let Some(spec) = attachment_spec("Scoped Sight") else {
        return;
    };
    let member = add_member(
        &mut app,
        "content/attachments/scoped_sight.attachment.ron",
        spec,
    );
    let folder = add_folder(&mut app, std::slice::from_ref(&member));
    app.world_mut()
        .insert_resource(ActiveAttachmentsFolderHandle::new(folder));
    app.world_mut()
        .insert_resource(AttachmentRegistry::default());
    app.world_mut()
        .write_message(AssetEvent::Modified { id: member.id() });

    let captured = capture_logs(|| {
        let result = app
            .world_mut()
            .run_system_once(redrive_attachments_on_asset_event);
        assert!(result.is_ok(), "the redrive system must run cleanly");
    });

    assert!(
        captured
            .iter()
            .any(|line| line.contains("attachment hot-reload")
                && line.contains("AttachmentRegistry")),
        "the attachment hot-reload must emit an info! line naming what reloaded; \
         captured: {captured:?}",
    );
}
