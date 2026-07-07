//! The GTW-375 sheet-IMAGE hot-reload reaction: react to a re-saved sheet `.png`
//! and re-prepare the terrain materials that sample it; plus its registrar.

use bevy::{
    asset::{AssetEvent, AssetId, Assets},
    image::Image,
    prelude::*,
};

use super::atlases::{SheetRole, TopDownAtlases};
use crate::TerrainFogMaterial;

/// `Update` (unguarded; self-gates on its [`Option`] borrows): live-reload ANY sprite
/// sheet registered in [`TopDownAtlases`] when its `.png` is re-saved (GTW-375 C4) —
/// terrain, characters, effects, portraits, or any future sheet, not just terrain.
///
/// The image asset itself is re-decoded into the SAME [`Handle<Image>`] by Bevy's
/// file-watcher, so the GPU texture refreshes on its own. It reads the [`MessageReader`] of
/// [`AssetEvent`](bevy::asset::AssetEvent)`<`[`Image`]`>` — asset events are MESSAGES in Bevy
/// 0.19, so this is a `MessageReader`, not an `EventReader` (`bevy-traps.md` #4) — and for
/// each [`Modified`](bevy::asset::AssetEvent::Modified):
///
/// - logs ONE `info!` per DISTINCT reloaded sheet in [`TopDownAtlases`], naming it by its
///   asset path (GTW-375 C5; events for ids that are not a loaded sheet — portrait nodes,
///   font atlases, one-off textures — are not logged); and
/// - re-prepares every [`TerrainFogMaterial`] whose `image` IS a modified id: one
///   [`Assets::get_mut`] deref per affected material queues the `AssetEvent::Modified`
///   that rebuilds its bind group against the fresh `GpuImage` (GTW-666 — the successor
///   of the retired GTW-665 registry poke, which forced a full despawn+respawn redraw;
///   the tiles now keep their entities and only the materials actually sampling the
///   reloaded image re-prepare).
///
/// Why ONLY the terrain materials need this (the Research-phase bevy-expert finding, Bevy
/// 0.19): the terrain draws through a custom [`Material2d`](bevy::sprite_render::Material2d)
/// (`TerrainFogMaterial`), whose `PreparedMaterial2d` bind group is a SNAPSHOT of the
/// `texture_view` baked at `as_bind_group` time — an image reload updates the `GpuImage` but
/// the existing bind group still references the OLD view, so the material must re-prepare.
/// The OTHER sheets draw as atlas SPRITES (gangers, effects, stair/ladder, and the portrait
/// UI node): the sprite pipeline keys its image bind group by [`AssetId<Image>`] and
/// invalidates+rebuilds it from the fresh `GpuImage` automatically on the same
/// `AssetEvent::Modified` — so those sheets show new pixels with ZERO system action, and
/// this system only LOGS them.
///
/// Guarded so it never panics. The [`MessageReader<AssetEvent<Image>>`](MessageReader) is
/// itself wrapped in an [`Option`] because `Messages<AssetEvent<Image>>` exists only when
/// the [`Image`] asset is registered (`ImagePlugin` / `DefaultPlugins`): a headless harness
/// that wires the renderer with an [`AssetServer`] but no image-asset stack would otherwise
/// trip Bevy's param validation (`Message not initialized`). When the buffer is absent the
/// param resolves to [`None`] and the system no-ops (nothing to drain — there is no buffer).
/// [`TopDownAtlases`] (the id→sheet map) is an [`Option`]al borrow used ONLY for the
/// logging; the [`Assets<TerrainFogMaterial>`] store is [`Option`]al too (absent without
/// the material plugin) and touched ONLY when a modified id is actually sampled — an
/// unrelated reload leaves the store's tick untouched (the `ResMut` binding never derefs).
///
/// Param-only (`bevy-traps.md` #7): the optional [`MessageReader`], the optional
/// [`TopDownAtlases`] / [`Assets<TerrainFogMaterial>`] borrows.
pub fn redrive_sheet_images_on_asset_event(
    events: Option<MessageReader<AssetEvent<Image>>>,
    atlases: Option<Res<TopDownAtlases>>,
    materials: Option<ResMut<Assets<TerrainFogMaterial>>>,
) {
    let Some(mut events) = events else {
        // No `Messages<AssetEvent<Image>>` buffer (no image-asset stack) — nothing to read
        // or drain; a real dev binary always has it via `ImagePlugin`/`DefaultPlugins`.
        return;
    };

    // The DISTINCT image ids modified this frame (several events can arrive for one id).
    let mut modified: Vec<AssetId<Image>> = Vec::new();
    for event in events.read() {
        let AssetEvent::Modified { id } = event else {
            continue;
        };
        if !modified.contains(id) {
            modified.push(*id);
        }
    }
    if modified.is_empty() {
        return;
    }

    // GTW-374 Part C convention / GTW-375 C5: log EVERY sheet hot-reload path, one line per
    // DISTINCT reloaded sheet, naming it by its asset path. Non-sheet ids (a portrait node,
    // a font atlas, a one-off texture, …) are not logged — reloading them must not claim a
    // sheet refreshed.
    if let Some(atlases) = atlases {
        let mut reloaded: Vec<SheetRole> = Vec::new();
        for id in &modified {
            let Some(role) = atlases.sheet_role_for_image(*id) else {
                continue;
            };
            if !reloaded.contains(&role) {
                reloaded.push(role);
            }
        }
        for role in &reloaded {
            info!(
                "tileset hot-reload: reloaded sheet `{}`, refreshing it",
                role.asset_path(),
            );
        }
    }

    // GTW-666: re-prepare every terrain material sampling a reloaded image — its bind
    // group is a snapshot of the OLD texture view (see the system doc), and only a
    // material-asset Modified rebuilds it. Collect through the immutable Deref (no
    // resource tick when nothing matches), then take one real DerefMut per affected
    // material: Bevy 0.19 queues `AssetEvent::Modified` on actual DerefMut only.
    let Some(mut materials) = materials else {
        return;
    };
    let stale: Vec<AssetId<TerrainFogMaterial>> = materials
        .iter()
        .filter(|(_, material)| modified.contains(&material.image.id()))
        .map(|(id, _)| id)
        .collect();
    for id in stale {
        if let Some(mut material) = materials.get_mut(id) {
            // The deref IS the re-prepare signal: same field values, fresh bind group.
            let _: &mut TerrainFogMaterial = &mut material;
        }
    }
}

/// Registers the sheet-IMAGE hot-reload reaction ([`redrive_sheet_images_on_asset_event`])
/// in an ungated `Update` — the one NON-RON hot-reload registration, relocated here with
/// its owning module when GTW-564 erased the presenter's `register_ron_tables` wall (the
/// RON chains now register through the generic hot-RON seam at their own modules).
///
/// Needs no `AssetServer` gate: the system self-guards on its `Option`al
/// `MessageReader<AssetEvent<Image>>` (the buffer only exists with the image-asset stack)
/// and its `Option`al resource borrows, so a headless app is a harmless no-op.
pub(crate) fn register_sheet_image_redrive(app: &mut App) {
    app.add_systems(Update, redrive_sheet_images_on_asset_event);
}
