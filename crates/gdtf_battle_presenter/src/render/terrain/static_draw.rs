//! The static-battlefield one-shot draw + its redraw triggers, and the
//! [`TerrainSprite`] marker every drawn tile carries.

use bevy::{
    camera::visibility::RenderLayers,
    ecs::template::template,
    math::primitives::Rectangle,
    prelude::*,
    scene::{CommandsSceneExt, bsn, template_value},
};
use gdtf_battle_sim::{
    battle::BattleReady,
    occupancy::{GRID_HEIGHT, GRID_WIDTH},
    prelude::{Cell, CellLevel, Level},
};

use super::{
    active_level::{ActiveLevel, ViewMode},
    band::{drawn_band, level_band},
    restamp::StampedGraphic,
    static_map::{SpriteResolveCtx, StaticMap, graphic_name_at, i32_extent, storey_has_terrain},
    treatment::{IsolateView, StoreyViewMode},
};
use crate::{TerrainFogMaterial, cell_to_world};

/// Marker tagging every static-terrain sprite this slice spawns.
///
/// A value-free marker (the no-bare-types marker carve-out) so a redraw / cover-swap
/// despawns exactly the terrain sprites — and ONLY them, never the S5 ganger sprites,
/// never the S2 [`WorldCamera`](crate::WorldCamera). It carries the source
/// [`CellLevel`] (the named sim newtype, never a bare key) so the
/// [`CoverDestroyed`](gdtf_battle_sim::occupancy_sync::CoverDestroyed) reaction can find the one sprite
/// at the smashed cell.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TerrainSprite {
    /// The `(cell, level)` this terrain sprite was drawn for.
    pub at: CellLevel,
}

/// `Update` (`PresenterSystems::Scene`, gated `resource_exists::<BattleInProgress>`): the
/// static-battlefield ONE-SHOT draw + redraw-on-level-change.
///
/// Fires when ANY of: a [`BattleReady`](gdtf_battle_sim::battle::BattleReady) drained this update,
/// [`ActiveLevel`] `is_changed()`, [`ViewMode`] `is_changed()` (GTW-521 — the full-view
/// toggle widens/narrows the [`drawn_band`] ceiling, so the terrain must redraw for the new
/// band), OR [`IsolateView`] `is_changed()` (GTW-594 — the Isolate toggle moves the band's
/// FLOOR). A sprite-def hot-reload is NOT a redraw trigger since GTW-666: a
/// `content/sprites/*.spritedef.ron` re-save rebuilds the
/// [`SpriteDefRegistry`](gdtf_content_families::sprites::SpriteDefRegistry) through the
/// family redrive and the sibling
/// [`restamp_tiles_on_def_change`](super::restamp::restamp_tiles_on_def_change)
/// re-resolves every ALREADY-DRAWN tile in place (tick-quiet for unchanged defs), and an
/// `alt_tileset_terrain.png` re-save re-prepares the affected tile materials directly
/// (`redrive_sheet_images_on_asset_event`) — either way the rendered tiles swap, live,
/// with no restart and no despawn. It despawns ALL existing [`TerrainSprite`]
/// entities, then (GTW-519) for the whole DRAWN BAND [`drawn_band`] (`0..=active`,
/// BOTTOM-UP) scans `0..GRID_WIDTH` × `0..GRID_HEIGHT` per storey and spawns one terrain
/// tile per NON-EMPTY cell as a shared unit-rect [`Mesh2d`] +
/// [`MeshMaterial2d<TerrainFogMaterial>`] (GTW-348 — the material path so EXPLORED can render
/// greyscale; the `Sprite` pipeline cannot desaturate) at
/// [`cell_to_world`](crate::cell_to_world) plus the def's ANCHOR offset (GTW-665 C2 —
/// the authored ground-contact/pivot sits ON the cell position; every seeded def
/// authors the center anchor, so shipped art draws exactly where it always did), on the
/// [`WORLD_RENDER_LAYER`](crate::WORLD_RENDER_LAYER), with the [`TerrainSprite`] marker. The
/// ground floor (storey 0) draws its FULL floor field (the pre-GTW-519 single-storey
/// behaviour); every UPPER storey in the band draws ONLY cells with a real terrain fact
/// ([`storey_has_terrain`]) so an open/empty upper cell emits nothing and the storey beneath
/// PEEKS THROUGH (C2). Everything strictly ABOVE `active` is culled (the band never enters
/// it). The per-storey Z ([`z_for`](crate::cell_to_world) via [`cell_to_world`]) gives the
/// painter's-algorithm occlusion (a higher storey draws in front) with NO new Z math (C3).
/// The despawn-first step makes the first-ready double-fire (a `BattleReady` on the same
/// update `ActiveLevel` first reads `is_changed`) idempotent, and a level cycle redraws the
/// whole `[0..=active]` band (C7).
///
/// GTW-665 (C1 — the def-driven resolution): each cell's PIXELS resolve from the
/// SIM-SPAWNED terrain entity's per-def
/// [`TerrainGraphicKey`](gdtf_battle_sim::piece::TerrainGraphicKey) FIRST (via
/// [`graphic_name_at`] → [`SpriteResolveCtx::resolved`] over the
/// [`SpriteDefRegistry`](gdtf_content_families::sprites::SpriteDefRegistry)), falling
/// back to the [`TileRole`](super::roles::TileRole) key mapped from the cell's
/// [`TerrainKind`](gdtf_battle_sim::occupancy::TerrainKind) only for the floor field (no
/// spawned entity). A name resolving NO def draws the LOUD magenta
/// [`MissingTileTexture`](super::resolve::MissingTileTexture) marker (C4 — warned, never
/// invisible, never a panic). So two
/// `Cover` defs whose `graphic_name`s differ (e.g. `"cover"` vs `"rubble"`) draw DISTINCT
/// sprites — the per-def graphic a kind-keyed default cannot express. The slab-only
/// OPTIONAL [`FootfallSound`](gdtf_battle_sim::piece::FootfallSound) is also read here from
/// the def's presenter facts; an absent footfall is
/// handled (no panic) with a documented silent default — there is no footfall-audio system
/// yet (guns-only).
///
/// Param-only (`bevy-traps.md` #7): [`Commands`], the [`StaticMap`] sim-grid bundle,
/// the [`SpriteResolveCtx`] resolution bundle (registry + asset server + images + the
/// C4 marker — GTW-665), [`Res<ActiveLevel>`], the
/// asset stores it builds tiles from ([`ResMut<Assets<TerrainFogMaterial>>`] for the
/// per-tile material, [`ResMut<Assets<Mesh>>`] + a [`Local`] cache for the shared
/// unit-rect quad), [`MessageReader<BattleReady>`], and the [`TerrainSprite`] despawn
/// query.
#[expect(
    clippy::too_many_arguments,
    reason = "the GTW-348 material path adds two asset stores (TerrainFogMaterial, Mesh) \
              to the existing draw params and GTW-665 adds the bundled resolution ctx; \
              further grouping into SystemParam bundles would not reduce the count and \
              would obscure the per-arg docs"
)]
pub fn draw_static_battlefield(
    mut commands: Commands,
    map: StaticMap,
    resolve: SpriteResolveCtx,
    active: Res<ActiveLevel>,
    view: Res<ViewMode>,
    isolate: Res<IsolateView>,
    mut materials: ResMut<Assets<TerrainFogMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut quad: Local<Option<Handle<Mesh>>>,
    mut ready: MessageReader<BattleReady>,
    existing: Query<Entity, With<TerrainSprite>>,
) {
    // The redraw triggers: a drained BattleReady (one-shot), an ActiveLevel change, or a
    // band toggle. A sprite-def / sheet hot-reload is NOT one (GTW-666): the registry
    // rebuild restamps the ALREADY-DRAWN tiles in place via the sibling
    // restamp_tiles_on_def_change (mutate-not-respawn, tick-quiet for unchanged defs),
    // and a sheet `.png` re-save re-prepares the affected materials directly
    // (redrive_sheet_images_on_asset_event). present_fog runs in the Compose stage,
    // chained after this Scene stage, so the fog re-applies to the redrawn tiles.
    // Fully DRAIN the reader (`.count()`, not `.next()`) so a multi-message ready never
    // leaves an unread BattleReady to re-fire a redundant redraw next update.
    let ready_fired = ready.read().count() > 0;
    // GTW-521: a ViewMode toggle changes the drawn_band ceiling, so it must redraw the terrain
    // for the new band exactly as an ActiveLevel change does. GTW-594: an IsolateView flip
    // changes the band's FLOOR (and preempts the two-state mode), so it is a fourth trigger.
    if !ready_fired && !active.is_changed() && !view.is_changed() && !isolate.is_changed() {
        return;
    }

    // Despawn ALL existing terrain sprites first — idempotent (a same-update
    // ready+is_changed double-fire redraws once) and level-scoped (off-active-level
    // terrain is gone after a level change).
    for entity in &existing {
        commands.entity(entity).despawn();
    }

    // The shared unit-rect quad (Rectangle::from_size(1×1), the same mesh Bevy's
    // SpriteMeshPlugin builds): added ONCE, then reused for every tile. The per-tile
    // TerrainFogMaterial's vertex_scale (= custom_size) sizes the quad in the shader.
    let mesh = quad
        .get_or_insert_with(|| meshes.add(Rectangle::from_size(Vec2::ONE)))
        .clone();

    // GTW-493: build the per-cell terrain presentation-fact map ONCE per draw — the
    // sim-spawned `(cell, level)` -> (per-def TerrainGraphicKey, optional slab FootfallSound).
    // Each cell then resolves its sprite def from its OWN graphic name (resolve_cell_sprite),
    // not solely from the TerrainKind-keyed TileRole default.
    let graphic_facts = map.graphic_facts();

    // GTW-519: draw the whole storey band 0..=active BOTTOM-UP (the UFO:EU multi-level
    // display) rather than the single active storey — the band lives in ONE `drawn_band`
    // helper the swap reactions share (C1/C6). Iterating ascending keeps the loop reading
    // bottom-up; the painter's-algorithm occlusion is the existing per-storey Z (z_for via
    // cell_to_world), NOT new math — a higher storey's tile carries a strictly greater z, so
    // it draws in front. A storey strictly above active is never entered, so it emits nothing
    // (the hard cull). present_fog dims each lower storey's tiles per-tile afterward.
    for level in level_band(drawn_band(*active, StoreyViewMode::new(*view, *isolate))) {
        for y in 0..i32_extent(GRID_HEIGHT) {
            for x in 0..i32_extent(GRID_WIDTH) {
                let cell = Cell::new(x, y);
                let key = CellLevel::new(cell, level);
                // GTW-493: read the slab footfall from the def's presenter_kind for this
                // cell. It is Slab-ONLY and OPTIONAL: a Wall/Cover entity carries no
                // FootfallSound, and a Slab def may omit it. ABSENT footfall is handled here
                // with NO panic and a DOCUMENTED default — there is NO footfall-audio system
                // yet (the sim stays guns-only), so a present key is logged at `debug` (so a
                // future footfall pass has a wired seam to consume) and an absent one is the
                // SILENT default (no clip).
                if let Some((_graphic, Some(footfall))) = graphic_facts.get(&key) {
                    let footfall_key: &str = footfall;
                    debug!(
                        "terrain footfall present at {key:?}: `{footfall_key}` (no \
                         footfall-audio system yet — default: silent)",
                    );
                }
                // GTW-519 PEEK-THROUGH (C2): a cell with nothing on it still resolves the
                // FLOOR default — but the caller only spawns a tile where a sim FACT exists
                // on THIS storey. An open/empty upper-storey cell (no terrain entity, no
                // slab, occupancy `Open`) resolves to floor yet has no fact keying it here
                // for a non-zero storey, so it emits NOTHING and the storey beneath peeks
                // through. The floor field is authored on storey 0 (the ground plane); upper
                // storeys draw only their real walls / cover / slabs.
                if level != Level::new(0) && !storey_has_terrain(&key, &graphic_facts, &map) {
                    continue;
                }
                // GTW-665: resolve the cell's graphic NAME (per-def graphic FIRST, the
                // TileRole key for the floor field) through the ONE def resolution — the
                // material carries the def's texture + rect (C1), the offset its anchor
                // (C2 — zero for the seeded center anchors, so shipped art is
                // pixel-identical), and a missing def yields the LOUD magenta marker (C4).
                let name = graphic_name_at(&key, &graphic_facts, &map);
                let (material, offset) = resolve.resolved(name, &key);
                let mesh2d = Mesh2d(mesh.clone());
                let material2d = MeshMaterial2d(materials.add(material));
                let transform =
                    Transform::from_translation(cell_to_world(cell, level) + offset.extend(0.0));
                let layers = RenderLayers::layer(crate::WORLD_RENDER_LAYER);
                // GTW-348 — terrain moved from the `Sprite` path to a `Mesh2d` +
                // `MeshMaterial2d<TerrainFogMaterial>` so EXPLORED cells can render GREYSCALE
                // (the sprite pipeline's per-channel multiply tint cannot desaturate).
                // Authored via `spawn_scene` (GTW-322): `Mesh2d` / `MeshMaterial2d` each wrap
                // a `Handle<_>`, which is NOT `Unpin` (so it has no `Template` impl) — exactly
                // like the old atlas `Sprite` — so each rides the `template(move |_|
                // Ok(value.clone()))` closure escape hatch (the `FnTemplate` output carries no
                // `Unpin` bound), the same one the AREA-1 widget builders use. The runtime
                // `Transform` / `RenderLayers` ARE `Clone + Default + Unpin`, so each rides
                // `template_value` (a value-overwrite). The `TerrainSprite` marker carries the
                // runtime source `CellLevel` (no `Default`), so it is `.insert`ed after the
                // scene. The tile is never read back by id (the redraw / cover-swap / fog find
                // it via the `TerrainSprite { at }` query, not a captured handle), so the
                // deferred materialization is inert — the same entity + components result.
                // The GTW-666 StampedGraphic records WHICH key the pixels came from, so
                // a def hot-reload restamps exactly what the tile shows.
                commands
                    .spawn_scene((
                        bsn! { template(move |_| Ok(mesh2d.clone())) },
                        bsn! { template(move |_| Ok(material2d.clone())) },
                        template_value(transform),
                        template_value(layers),
                    ))
                    .insert((TerrainSprite { at: key }, StampedGraphic::from_key(name)));
            }
        }
    }
}
