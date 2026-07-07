//! The GTW-375 sheet-IMAGE hot-reload reaction: react to a re-saved sheet `.png`
//! and (for the terrain sheet) force the terrain redraw; plus its registrar.

use bevy::{asset::AssetEvent, prelude::*};
use gdtf_content_families::sprites::SpriteDefRegistry;

use super::atlases::{SheetRole, TopDownAtlases};

/// `Update` (unguarded; self-gates on its [`Option`] borrows): live-reload ANY sprite
/// sheet registered in [`TopDownAtlases`] when its `.png` is re-saved (GTW-375 C4) —
/// terrain, characters, effects, portraits, or any future sheet, not just terrain.
///
/// The image asset itself is re-decoded into the SAME [`Handle<Image>`] by Bevy's
/// file-watcher, so the GPU texture refreshes on its own. It reads the [`MessageReader`] of
/// [`AssetEvent`](bevy::asset::AssetEvent)`<`[`Image`]`>` — asset events are MESSAGES in Bevy
/// 0.19, so this is a `MessageReader`, not an `EventReader` (`bevy-traps.md` #4) — and for
/// each [`Modified`](bevy::asset::AssetEvent::Modified) maps the image id back to its sheet
/// via [`TopDownAtlases::sheet_role_for_image`]. It collects the DISTINCT reloaded
/// [`SheetRole`]s (ignoring events for ids that are not a loaded sheet — portrait nodes,
/// font atlases, one-off textures) and:
///
/// - logs ONE `info!` per reloaded sheet, naming it by its asset path (GTW-375 C5); and
/// - if [`Terrain`](SheetRole::Terrain) is among them, calls
///   [`DetectChangesMut::set_changed`] on the [`SpriteDefRegistry`] (the GTW-665 redraw
///   signal — the retired `TileRoles` poke re-anchored) to force
///   `draw_static_battlefield`'s `defs_changed()` trigger to despawn+respawn the terrain
///   tiles against the freshly-reloaded texture.
///
/// Why ONLY terrain gets the poke (the Research-phase bevy-expert finding, Bevy 0.19): the
/// terrain draws through a custom [`Material2d`](bevy::sprite_render::Material2d)
/// (`TerrainFogMaterial`), whose `PreparedMaterial2d` bind group is a SNAPSHOT of the
/// `texture_view` baked at `as_bind_group` time — an image reload updates the `GpuImage` but
/// the existing bind group still references the OLD view, so the only way to re-bind is to
/// re-prepare the material, which the despawn+respawn in `draw_static_battlefield` does
/// (`materials.add(...)` mints fresh `PreparedMaterial2d` entries against the already-updated
/// `GpuImage`). The OTHER sheets draw as atlas SPRITES (gangers, effects, stair/ladder, and
/// the portrait UI node): the sprite pipeline keys its image bind group by
/// [`AssetId<Image>`] and invalidates+rebuilds it from the fresh `GpuImage` automatically on
/// the same `AssetEvent::Modified` — so those sheets show new pixels with ZERO system action,
/// and this system only LOGS them.
///
/// Guarded so it never panics. The [`MessageReader<AssetEvent<Image>>`](MessageReader) is
/// itself wrapped in an [`Option`] because `Messages<AssetEvent<Image>>` exists only when
/// the [`Image`] asset is registered (`ImagePlugin` / `DefaultPlugins`): a headless harness
/// that wires the renderer with an [`AssetServer`] but no image-asset stack would otherwise
/// trip Bevy's param validation (`Message not initialized`). When the buffer is absent the
/// param resolves to [`None`] and the system no-ops (nothing to drain — there is no buffer).
/// [`TopDownAtlases`] (the id→sheet map) is an [`Option`]al borrow, draining the reader and
/// returning early when it is missing (`bevy-traps.md` #1) so a pre-resolve event does not
/// linger and re-fire later. The [`SpriteDefRegistry`] is [`Option`]al too and used ONLY
/// for the terrain poke, so a non-terrain reload still LOGS even when it is absent.
///
/// Param-only (`bevy-traps.md` #7): the optional [`MessageReader`], the optional
/// [`TopDownAtlases`] / [`SpriteDefRegistry`] borrows.
pub fn redrive_sheet_images_on_asset_event(
    events: Option<MessageReader<AssetEvent<Image>>>,
    atlases: Option<Res<TopDownAtlases>>,
    defs: Option<ResMut<SpriteDefRegistry>>,
) {
    let Some(mut events) = events else {
        // No `Messages<AssetEvent<Image>>` buffer (no image-asset stack) — nothing to read
        // or drain; a real dev binary always has it via `ImagePlugin`/`DefaultPlugins`.
        return;
    };
    let Some(atlases) = atlases else {
        // Drain the reader so a pre-resolve event does not linger and re-fire once the
        // atlas resource arrives; there is no id→sheet map to consult yet.
        events.clear();
        return;
    };

    // Map every Modified event to the sheet it reloaded, keeping the DISTINCT roles so each
    // sheet is logged once even if several events arrive for it this frame.
    let mut reloaded: Vec<SheetRole> = Vec::new();
    for event in events.read() {
        let AssetEvent::Modified { id } = event else {
            continue;
        };
        let Some(role) = atlases.sheet_role_for_image(*id) else {
            // Not a loaded sheet (a portrait node, a font atlas, a one-off texture, …) —
            // ignore it; a non-sheet reload must not log or redraw a sheet.
            continue;
        };
        if !reloaded.contains(&role) {
            reloaded.push(role);
        }
    }

    if reloaded.is_empty() {
        return;
    }

    for role in &reloaded {
        // GTW-374 Part C convention / GTW-375 C5: log EVERY hot-reload path, one line per
        // reloaded sheet, naming it by its asset path.
        info!(
            "tileset hot-reload: reloaded sheet `{}`, refreshing it",
            role.asset_path(),
        );
    }

    // ONLY the terrain sheet needs an explicit redraw poke: it draws through the custom
    // TerrainFogMaterial whose bind group is a snapshot, so force draw_static_battlefield's
    // `defs_changed()` trigger to despawn+respawn the tiles against the fresh GPU
    // texture. The other sheets are atlas sprites and refresh through the sprite pipeline on
    // their own (see the system doc). The resolved defs are unchanged, so this marks the
    // SpriteDefRegistry changed WITHOUT mutating it (GTW-665 — the retired TileRoles poke
    // re-anchored) — and is gated on the registry being present, so a non-terrain reload
    // still LOGS above even when it is absent.
    if reloaded.contains(&SheetRole::Terrain)
        && let Some(mut defs) = defs
    {
        defs.set_changed();
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
