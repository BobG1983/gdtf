//! The px/coordinate bridge definitions: [`CELL_PX`], the [`cell_to_world`] projection,
//! the role-keyed atlas resource, and the atlas-load system.

use bevy::{
    asset::AssetEvent,
    image::{ImageLoaderSettings, ImageSampler, TextureAtlasLayout},
    platform::collections::HashMap,
    prelude::*,
};
use gdtf_battle_sim::{Cell, Level, SimPos};

use crate::TileRoles;

/// On-screen size of one cell, in world units.
///
/// The ONE presenter source of truth for cell size. A `const`, NOT a domain newtype
/// — the framework-plumbing carve-out (`.claude/rules/no-bare-types.md` clause 4):
/// a scalar fed straight to a [`Transform`] / `custom_size`, not a domain quantity,
/// the same reasoning the landed `WORLD_RENDER_LAYER`-class consts use. 16.0 because
/// the source tiles are 16×16 px, so one source tile maps to a 16-world-unit cell.
pub const CELL_PX: f32 = 16.0;

/// Per-level world-space draw-z spacing, in world units.
///
/// A small monotonic gap between storeys so sprites on different levels do not
/// z-fight. Full multi-level z-stacking is GTW-49 / GTW-10; this slice only needs a
/// stable per-level z for the projection.
const Z_PER_LEVEL: f32 = 1.0;

/// The within-storey draw-z bias that lifts a ganger sprite ABOVE its own floor tile.
///
/// A `const`, NOT a domain newtype — the `CELL_PX`-class framework-plumbing carve-out
/// (`.claude/rules/no-bare-types.md` clause 4): a scalar fed straight to a
/// [`Transform`]'s `z`, not a domain quantity. Strictly `< Z_PER_LEVEL` (`0.1 < 1.0`)
/// so a ganger's lifted z never sorts into the NEXT storey's band — it stays within its
/// own storey, just in front of the same-cell terrain (which draws at the bare level z,
/// [`Layer::Terrain`] = `0.0`). This fixes the GTW-283 occlusion: at storey 0 the ganger
/// and its floor both projected to `z = 0.0`, and Bevy 0.18's non-deterministic same-z 2D
/// sort let the opaque floor draw over the ganger. `0.1` is the smallest legible lift.
pub const GANGER_Z_BIAS: f32 = 0.1;

/// A presenter draw layer within a single storey — the ONE place the
/// terrain &lt; vertical-link &lt; fire-target &lt; actor &lt; highlight &lt; path-preview
/// stacking order lives.
///
/// A domain value (a real named type, not a bare z magnitude), per
/// `.claude/rules/no-bare-types.md`. Each layer's [`z_bias`](Layer::z_bias) is added on
/// top of the per-storey level z by [`cell_to_world_layered`] so a sprite draws in front
/// of the lower layers at its own cell without crossing into the next storey's band (every
/// bias is strictly `< Z_PER_LEVEL`). This slice wires [`Terrain`](Layer::Terrain) (the
/// bare level z, via [`cell_to_world`]) and [`Actor`](Layer::Actor) (the
/// [`GANGER_Z_BIAS`] lift); [`Highlight`](Layer::Highlight) is defined so the documented
/// order is complete, but routing the hover/selection highlight through it is the
/// in-engine-adjustable later tweak the GTW-283 contract flags (it currently still draws
/// at the bare level z). [`PathPreview`](Layer::PathPreview) is the GTW-358 route-preview
/// band — the topmost within-storey band, drawn ABOVE the highlight so the previewed route
/// (and its target-cell TU-cost label, GTW-368) reads over the terrain, actors, and reticle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Layer {
    /// Floor / wall / cover terrain — the ground plane, drawn at the bare per-storey z.
    Terrain,
    /// The GTW-545 area-damage-field wash — a translucent hazard tile drawn OVER the terrain
    /// floor of a fielded cell (a toxic pool / electrified floor / burning ground), so the
    /// danger zone reads over its own floor tile. Drawn just ABOVE the terrain ground plane but
    /// strictly BELOW the [`VerticalLink`](Layer::VerticalLink) / [`Actor`](Layer::Actor) bands
    /// so a ganger (or a stair) standing on the field draws OVER the wash — it is a ground-plane
    /// hazard the unit stands in, not something that occludes the unit. Strictly `< GANGER_Z_BIAS
    /// < Z_PER_LEVEL`, so it never sorts into the actor band or the next storey's band.
    Field,
    /// A GTW-359 vertical-link (stair / ladder) endpoint tile — drawn just ABOVE the
    /// terrain ground plane (so the stair / ladder reads over its own floor tile) but
    /// strictly BELOW the [`Actor`](Layer::Actor) band so a ganger standing on the link
    /// cell draws over it. Strictly `< GANGER_Z_BIAS < Z_PER_LEVEL`, so it never sorts
    /// into the actor band or the next storey's band.
    VerticalLink,
    /// The GTW-371 fire-target highlight — the RED tile drawn UNDER the enemy a fireable
    /// hover would shoot. Drawn ABOVE the terrain + vertical-link bands but strictly BELOW
    /// the [`Actor`](Layer::Actor) band (`GANGER_Z_BIAS * 0.75 = 0.075 < GANGER_Z_BIAS`),
    /// so it renders UNDER the enemy's own sprite — the contract's "under the actor"
    /// treatment, exactly the way the vertical-link tile sits under a ganger standing on it.
    FireTarget,
    /// A ganger (actor) — drawn just in front of its own terrain by [`GANGER_Z_BIAS`].
    Actor,
    /// The hover / selection highlight — drawn in front of the actor so it tints the unit
    /// (documented order; wiring deferred, see the type doc).
    Highlight,
    /// The GTW-387 reachable-range overlay — the cells the selected ganger can reach
    /// within its remaining TU. Drawn ABOVE the highlight so the range tint reads over the
    /// terrain and actors, but strictly BELOW the [`PathPreview`](Layer::PathPreview) so
    /// the move-route still reads over the range highlight when both are shown.
    ReachableRange,
    /// The GTW-358 route-preview highlight — the previewed `find_path` route from the
    /// selected ganger to the target cell (and the GTW-368 target-cell TU-cost label), drawn
    /// strictly ABOVE the highlight so the route + its cost label read over the terrain,
    /// actors, and reticle. The topmost within-storey band — still strictly `< Z_PER_LEVEL`
    /// so it never sorts into the next storey's band.
    PathPreview,
}

impl Layer {
    /// This layer's within-storey draw-z bias, added on top of the per-storey level z.
    ///
    /// Strictly increasing terrain &lt; field &lt; vertical-link &lt; fire-target &lt; actor &lt;
    /// highlight &lt; reachable-range &lt; path-preview, and every value is strictly `<
    /// Z_PER_LEVEL` so a biased sprite never sorts into the next storey's band.
    #[must_use]
    const fn z_bias(self) -> f32 {
        match self {
            Self::Terrain => 0.0,
            // GTW-545: the field hazard wash — just above the terrain floor (reads over its own
            // floor tile), strictly below every other band so the ganger / stair standing IN the
            // field draws over the wash (`0.025 < GANGER_Z_BIAS`).
            Self::Field => GANGER_Z_BIAS * 0.25,
            // Above the terrain ground plane, strictly below the actor band so a ganger
            // standing on the stair / ladder draws over it (`0.05 < GANGER_Z_BIAS`).
            Self::VerticalLink => GANGER_Z_BIAS * 0.5,
            // GTW-371: above the terrain / vertical-link bands, strictly below the actor band
            // so the red fire-target tile renders UNDER the enemy's sprite
            // (`0.075 < GANGER_Z_BIAS`).
            Self::FireTarget => GANGER_Z_BIAS * 0.75,
            Self::Actor => GANGER_Z_BIAS,
            // Strictly above the actor, still within the storey band (`< Z_PER_LEVEL`).
            Self::Highlight => GANGER_Z_BIAS * 2.0,
            // GTW-387: above the highlight, below the route-preview so the move-route reads
            // over the range highlight when both are shown.
            Self::ReachableRange => GANGER_Z_BIAS * 2.5,
            // Topmost within-storey band — above the highlight + range overlay (the route +
            // its cost label read over the reticle and range tint), still `< Z_PER_LEVEL`.
            Self::PathPreview => GANGER_Z_BIAS * 3.0,
        }
    }
}

/// Projects a sim cell + level into the top-down renderer's world-space position.
///
/// Row 0 sits at the TOP: Bevy's +Y is up, so a larger `cell.y` (further down the
/// grid) yields a smaller world `y`. `x` grows right by exactly [`CELL_PX`] per cell.
/// The `z` is a stable per-level draw-z ([`z_for`]) so sprites on different storeys
/// do not z-fight; it is NOT full multi-level stacking (GTW-49 / GTW-10).
///
/// `cell.x` / `cell.y` read through [`Cell`]'s `Deref<Target = IVec2>`; the level
/// index reads through [`Level`]'s `Deref<Target = u8>` inside [`z_for`].
#[must_use]
pub fn cell_to_world(cell: Cell, level: Level) -> Vec3 {
    Vec3::new(
        cell.x as f32 * CELL_PX,
        -(cell.y as f32) * CELL_PX,
        z_for(level),
    )
}

/// Projects a CONTINUOUS sim-unit position ([`SimPos`]) into the top-down renderer's
/// world-space — the [`cell_to_world`] projection generalised to a fractional point.
///
/// Same mapping as [`cell_to_world`] (one sim unit = one [`CELL_PX`] cell; +Y is up, so a
/// larger `pos.y` yields a smaller world `y`), but for a continuous point rather than a
/// discrete `(cell, level)`: the GTW-290 muzzle origin is a [`SimPos`] (a fractional 3D
/// fire point), so the muzzle flash + tracer origin map through this. The `z` scales the
/// fractional storey by [`Z_PER_LEVEL`] (matching [`z_for`]'s discrete `*level *
/// Z_PER_LEVEL`) so a muzzle on storey *n* draws in that storey's band.
///
/// `pos.x` / `pos.y` / `pos.z` read through [`SimPos`]'s `Deref<Target = Vec3>`.
#[must_use]
pub fn sim_pos_to_world(pos: SimPos) -> Vec3 {
    Vec3::new(pos.x * CELL_PX, -pos.y * CELL_PX, pos.z * Z_PER_LEVEL)
}

/// Projects a sim cell + level into world-space, lifted by `layer`'s within-storey
/// draw-z bias — the [`cell_to_world`] position with [`Layer::z_bias`] added to `z`.
///
/// The ONE place a presenter draws a sprite "in front of" the lower layers at the same
/// cell: [`Terrain`](Layer::Terrain) sits at the bare per-storey z (equivalent to
/// [`cell_to_world`]), [`Actor`](Layer::Actor) is lifted by [`GANGER_Z_BIAS`] so a ganger
/// draws over its own floor tile (GTW-283), and the lift never crosses into the next
/// storey (every bias is strictly `< Z_PER_LEVEL`). `x` / `y` are unchanged from
/// [`cell_to_world`].
#[must_use]
pub fn cell_to_world_layered(cell: Cell, level: Level, layer: Layer) -> Vec3 {
    let mut world = cell_to_world(cell, level);
    world.z += layer.z_bias();
    world
}

/// The world-space draw-z for a storey `level`.
///
/// Monotonic in the storey index so higher storeys draw in front: `*level` (read
/// through [`Level`]'s `Deref<Target = u8>`) scaled by [`Z_PER_LEVEL`]. Kept private
/// — callers use [`cell_to_world`].
pub(super) fn z_for(level: Level) -> f32 {
    f32::from(*level) * Z_PER_LEVEL
}

/// Which of the role-separated sprite sheets an atlas entry belongs to.
///
/// A domain value (a real named type, not a bare string key), per
/// `.claude/rules/no-bare-types.md`. This loads the three 16×16 render sheets the
/// playable slices consume PLUS the GTW-278 [`Portraits`](SheetRole::Portraits) face
/// sheet (32×32 cells) the status / hover panels read; the remaining deferred `ui` /
/// `items` sheets join this enum in their consuming slices via the same mechanism. The
/// per-sheet tile size is NOT hardcoded — each role declares its own [`tile_px`](SheetRole::tile_px)
/// (the render sheets stay 16, the portrait sheet is 32).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SheetRole {
    /// Terrain tiles — `sprites/alt_tileset_terrain.png` (the S4 draw sheet).
    Terrain,
    /// Character tiles — `sprites/alt_tileset_characters.png` (the S5 draw sheet).
    Characters,
    /// Effect tiles — `sprites/alt_tileset_effects.png` (the S6 draw sheet).
    Effects,
    /// Portrait faces — `sprites/alt_tileset_portraits.png` (the GTW-278 HUD sheet): a
    /// 10×10 grid of 32×32-px faces, indices `0..=99`. Read as a `bevy_ui` `ImageNode`
    /// atlas variant by the status / hover panels' shared stat block, NOT a world
    /// sprite (the UI layer, not the map). Unlike the render sheets its cells are 32 px
    /// ([`tile_px`](SheetRole::tile_px)).
    Portraits,
}

impl SheetRole {
    /// Every sheet this loader loads, in a fixed order.
    ///
    /// The three 16×16 render sheets plus the GTW-278 32×32 portrait sheet; the
    /// deferred `ui` / `items` sheets are still absent here.
    const ALL: [Self; 4] = [
        Self::Terrain,
        Self::Characters,
        Self::Effects,
        Self::Portraits,
    ];

    /// Loose-file path (relative to the asset source root) of this sheet's PNG.
    ///
    /// The path a working-dir-at-workspace-root app and a workspace-rooted test
    /// both resolve to the shipped sheet. `pub` so integration tests can call the
    /// SAME path the runtime `load_topdown_atlases` uses — the GTW-447 load-state
    /// proof uses this to assert the new `sprites/` paths resolve to
    /// `LoadState::Loaded`.
    #[must_use]
    pub const fn asset_path(self) -> &'static str {
        match self {
            Self::Terrain => "sprites/alt_tileset_terrain.png",
            Self::Characters => "sprites/alt_tileset_characters.png",
            Self::Effects => "sprites/alt_tileset_effects.png",
            Self::Portraits => "sprites/alt_tileset_portraits.png",
        }
    }

    /// This sheet's grid shape as `(columns, rows)` of [`tile_px`](SheetRole::tile_px)
    /// cells.
    ///
    /// terrain 16×22 (352 tiles), characters 16×18 (288), effects 16×8 (128) — all of
    /// 16×16 px — and the GTW-278 portraits 10×10 (100 faces) of 32×32 px; the
    /// `from_grid` dimensions for [`load_topdown_atlases`]. `pub` (GTW-566 C7) so the
    /// content editor's terrain tile atlas reads the SAME sheet spec the presenter
    /// draws with instead of mirroring these dimensions as its own consts; the sibling
    /// `topdown::test` module pins the per-sheet grid through it too.
    #[must_use]
    pub const fn grid(self) -> (u32, u32) {
        match self {
            Self::Terrain => (16, 22),
            Self::Characters => (16, 18),
            Self::Effects => (16, 8),
            Self::Portraits => (10, 10),
        }
    }

    /// This sheet's per-cell tile size, in source pixels — the square edge of one
    /// atlas cell.
    ///
    /// The render sheets are 16-px tiles; the GTW-278 portrait sheet is 32-px faces.
    /// Threaded into [`TextureAtlasLayout::from_grid`] per sheet by
    /// [`load_topdown_atlases`] so the portrait layout is NOT mis-sized to 16 (which
    /// would carve each 32-px face into four wrong sub-tiles). A `const`, NOT a domain
    /// newtype — the `CELL_PX`-class framework-plumbing carve-out
    /// (`.claude/rules/no-bare-types.md` clause 4): it is a layout dimension fed
    /// straight to `from_grid`, not a domain quantity. `pub` (GTW-566 C7) so the
    /// content editor's terrain tile atlas reads the SAME per-cell size the presenter
    /// draws with instead of mirroring it as its own const; the sibling
    /// `topdown::test` module pins the per-sheet tile size through it too.
    #[must_use]
    pub const fn tile_px(self) -> u32 {
        match self {
            Self::Terrain | Self::Characters | Self::Effects => 16,
            Self::Portraits => 32,
        }
    }

    /// This sheet's per-asset texture SAMPLER override, or [`None`] to keep the asset
    /// server's default (GTW-295).
    ///
    /// The render sheets ([`Terrain`](SheetRole::Terrain) /
    /// [`Characters`](SheetRole::Characters) / [`Effects`](SheetRole::Effects)) draw as world
    /// sprites at ~1:1 source-to-screen and do not bleed, so they keep the default sampler
    /// ([`None`]). The [`Portraits`](SheetRole::Portraits) sheet is upscaled into a `bevy_ui`
    /// portrait node much larger than its 32-px faces; the default LINEAR sampler bilinearly
    /// blends the transparent-white (255,255,255,0) rows inset at the top of each face into a
    /// whitish fringe — so it overrides to [`ImageSampler::nearest`] (point sampling, no
    /// interpolation across those rows). [`load_topdown_atlases`] feeds this into
    /// [`load_with_settings`](AssetServer::load_with_settings); `pub(super)` so the sibling
    /// `topdown::test` module can pin the per-sheet decision without an app harness (the loaded
    /// image's sampler is unreachable in the headless `no_renderer` config — the image asset
    /// never finishes decoding without a render device).
    pub(super) fn sampler_override(self) -> Option<ImageSampler> {
        match self {
            Self::Portraits => Some(ImageSampler::nearest()),
            Self::Terrain | Self::Characters | Self::Effects => None,
        }
    }
}

/// One render sheet's loaded handles: its image plus the atlas layout over it.
///
/// A NAMED type, per `.claude/rules/no-bare-types.md` — no bare `Handle<...>` stored
/// as a domain value. The S4/S5/S6 draw systems read these to build sprites (see the
/// module-level sprite recipe).
#[derive(Debug, Clone)]
pub struct SheetAtlas {
    /// The sheet image handle (loaded via [`AssetServer::load`]).
    pub image:  Handle<Image>,
    /// The grid layout over [`Self::image`], one entry per cell at the sheet's own
    /// tile size ([`SheetRole::tile_px`]).
    pub layout: Handle<TextureAtlasLayout>,
}

/// The presenter-owned, role-keyed atlas resource.
///
/// Holds, KEYED BY [`SheetRole`], the loaded [`SheetAtlas`] (image + layout handles)
/// for each render sheet. Built ONCE by [`load_topdown_atlases`] and present before
/// the S4/S5/S6 draw systems run, which read it to spawn sprites. A framework type
/// (`Resource`), exempt from no-bare-types; the role key it stores is the named
/// [`SheetRole`]. The GTW-278 [`Portraits`](SheetRole::Portraits) sheet rides the same
/// map — the status / hover panels read its image + layout to build a UI portrait node;
/// the deferred `ui` / `items` sheets join the same way.
#[derive(Resource, Debug, Clone)]
pub struct TopDownAtlases {
    /// One loaded [`SheetAtlas`] per loaded [`SheetRole`].
    sheets: HashMap<SheetRole, SheetAtlas>,
}

impl TopDownAtlases {
    /// The loaded [`SheetAtlas`] for `role`, if that role was loaded.
    ///
    /// Returns [`None`] for a role not in this loader's set (e.g. a deferred
    /// `ui` / `items` sheet) — callers match the [`Option`] rather than risk a panic.
    #[must_use]
    pub fn role(&self, role: SheetRole) -> Option<&SheetAtlas> {
        self.sheets.get(&role)
    }

    /// Which [`SheetRole`] (if any) the given image id belongs to.
    ///
    /// Scans the loaded sheets for the one whose [`SheetAtlas::image`] handle has this
    /// id, returning its role. The inverse of [`role`](Self::role): the image hot-reload
    /// (GTW-375 C4) reads an [`AssetEvent`](bevy::asset::AssetEvent)`<`[`Image`]`>` carrying
    /// only an [`AssetId<Image>`] and must map it back to the sheet that reloaded so it can
    /// log the sheet by name and (for [`Terrain`](SheetRole::Terrain)) force the terrain
    /// redraw. Returns [`None`] for an id that is not any loaded sheet's image (a portrait
    /// node, a one-off texture, a font atlas, …) so callers ignore unrelated reloads.
    #[must_use]
    pub fn sheet_role_for_image(&self, id: AssetId<Image>) -> Option<SheetRole> {
        self.sheets
            .iter()
            .find_map(|(role, sheet)| (sheet.image.id() == id).then_some(*role))
    }
}

/// Loads every sprite sheet and inserts the [`TopDownAtlases`] resource.
///
/// Runs ONCE (the [`TopDownRendererPlugin`](crate::TopDownRendererPlugin) schedules
/// it in [`Startup`]): for each [`SheetRole`] it loads the PNG via
/// [`AssetServer::load`] (NOT the RON loader — that is `.ron`-only) and builds one
/// [`TextureAtlasLayout::from_grid`] per sheet at the role's OWN tile size
/// ([`SheetRole::tile_px`] — 16 for the render sheets, 32 for the GTW-278 portrait
/// sheet) and `(cols, rows)`, adding each layout to [`Assets<TextureAtlasLayout>`].
/// The resource is inserted via [`Commands`] so it is present before the draw slices
/// run.
///
/// Per `.claude/rules/bevy-traps.md` #7 this takes only normal system params — no
/// `&mut World`: [`Commands`] for the resource insert, [`Res<AssetServer>`] for the
/// image loads, and [`ResMut<Assets<TextureAtlasLayout>>`] to register the layouts.
pub fn load_topdown_atlases(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    let mut sheets = HashMap::default();
    for role in SheetRole::ALL {
        let (columns, rows) = role.grid();
        let layout =
            TextureAtlasLayout::from_grid(UVec2::splat(role.tile_px()), columns, rows, None, None);
        sheets.insert(
            role,
            SheetAtlas {
                image:  load_sheet_image(&asset_server, role),
                layout: layouts.add(layout),
            },
        );
    }

    commands.insert_resource(TopDownAtlases { sheets });
}

/// Loads `role`'s sheet image, choosing the texture sampler per role (GTW-295).
///
/// The per-role sampler DECISION is [`role.sampler_override()`](SheetRole::sampler_override):
/// the render sheets keep the asset server's DEFAULT sampler ([`None`]), and the
/// [`Portraits`](SheetRole::Portraits) sheet overrides it to NEAREST so its upscale into the
/// HUD portrait node point-samples instead of bilinearly blending the tiles' transparent-white
/// top rows into a whitish fringe (the GTW-295 white-line fix). `from_grid` stays the
/// geometrically-correct atlas carving; only the SAMPLER changes.
fn load_sheet_image(asset_server: &AssetServer, role: SheetRole) -> Handle<Image> {
    match role.sampler_override() {
        // `load_with_settings` is deprecated in Bevy 0.19 in favor of the
        // `load_builder().with_settings(..).load(path)` chain.
        Some(sampler) => asset_server
            .load_builder()
            .with_settings(move |settings: &mut ImageLoaderSettings| {
                settings.sampler = sampler.clone();
            })
            .load(role.asset_path()),
        None => asset_server.load(role.asset_path()),
    }
}

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
///   [`DetectChangesMut::set_changed`] on [`TileRoles`] to force
///   `draw_static_battlefield`'s `roles.is_changed()` trigger to despawn+respawn the terrain
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
/// linger and re-fire later. [`TileRoles`] is [`Option`]al too and used ONLY for the terrain
/// poke, so a non-terrain reload still LOGS even when [`TileRoles`] is absent.
///
/// Param-only (`bevy-traps.md` #7): the optional [`MessageReader`], the optional
/// [`TopDownAtlases`] / [`TileRoles`] borrows.
pub fn redrive_sheet_images_on_asset_event(
    events: Option<MessageReader<AssetEvent<Image>>>,
    atlases: Option<Res<TopDownAtlases>>,
    roles: Option<ResMut<TileRoles>>,
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
    // `roles.is_changed()` trigger to despawn+respawn the tiles against the fresh GPU
    // texture. The other sheets are atlas sprites and refresh through the sprite pipeline on
    // their own (see the system doc). The tile indices are unchanged, so this marks TileRoles
    // changed WITHOUT mutating it — and is gated on TileRoles being present, so a non-terrain
    // reload still LOGS above even when TileRoles is absent.
    if reloaded.contains(&SheetRole::Terrain)
        && let Some(mut roles) = roles
    {
        roles.set_changed();
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

#[cfg(test)]
mod test {
    use std::sync::{Arc, Mutex};

    use bevy::{
        MinimalPlugins,
        asset::{AssetApp, AssetEvent, AssetPlugin, Assets, Handle},
        image::{Image, TextureAtlasLayout},
        log::{
            tracing::{
                Event, Subscriber,
                field::{Field, Visit},
                subscriber::with_default,
            },
            tracing_subscriber::{Layer, layer::Context, prelude::*, registry::Registry},
        },
        platform::collections::HashMap,
        prelude::*,
    };

    use super::{SheetAtlas, SheetRole, TopDownAtlases, redrive_sheet_images_on_asset_event};
    use crate::{TileIndex, TileRoles};

    /// A scoped `tracing` layer that records each event's `message` field — the minimal
    /// capture needed to prove the `info!` hot-reload line fired. Mirrors the GTW-374
    /// combat redrive test's / `roles.rs`'s `CaptureLayer` (no shared util is reachable here).
    struct CaptureLayer {
        /// The shared buffer captured messages append to.
        messages: Arc<Mutex<Vec<String>>>,
    }

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
            if let Some(message) = visitor.message
                && let Ok(mut buffer) = self.messages.lock()
            {
                buffer.push(message);
            }
        }
    }

    /// Run `body` with a scoped [`CaptureLayer`] active, returning every captured message.
    ///
    /// The subscriber is scoped to this call (`with_default`), so it never leaks into other
    /// tests; the returned `Vec` is in emission order.
    fn capture_logs(body: impl FnOnce()) -> Vec<String> {
        let messages: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        let layer = CaptureLayer {
            messages: Arc::clone(&messages),
        };
        let subscriber = Registry::default().with(layer);
        with_default(subscriber, body);
        messages
            .lock()
            .map(|buffer| buffer.clone())
            .unwrap_or_default()
    }

    /// Probe resource: the value of `TileRoles::is_changed()` observed by a downstream system
    /// the LAST time it ran. The image-redrive test reads this to prove the terrain redraw
    /// trigger (`set_changed()`) fired — a downstream `DetectChanges` witness, exactly the C8
    /// "assert `TileRoles` becomes `is_changed` via a downstream system" shape.
    #[derive(Resource, Default)]
    struct RolesChangedWitness {
        /// Whether `TileRoles` was `is_changed()` when the witness system last ran.
        changed: bool,
    }

    /// Downstream witness system: records whether `TileRoles` is currently `is_changed()`.
    ///
    /// Ordered `.after(redrive_sheet_images_on_asset_event)` so a `set_changed()` the redrive
    /// performs THIS frame is visible to it (change ticks compare against this system's own
    /// last-run tick). Overwrites the witness each frame (no latching) so the read after the
    /// event update reflects only that frame.
    fn witness_roles_changed(roles: Res<TileRoles>, mut witness: ResMut<RolesChangedWitness>) {
        witness.changed = roles.is_changed();
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
            .init_resource::<RolesChangedWitness>()
            .add_systems(
                Update,
                witness_roles_changed.after(redrive_sheet_images_on_asset_event),
            )
            .add_systems(Update, redrive_sheet_images_on_asset_event);
        app
    }

    /// A `TileRoles` with every field set to the same index — a valid fixture; the image
    /// redrive never reads the indices, only marks the resource changed.
    fn uniform_roles() -> TileRoles {
        let index = TileIndex::new(7);
        TileRoles {
            floor:                index,
            floor_alt_panel:      index,
            wall:                 index,
            wall_ew:              index,
            cover:                index,
            emplacement:          index,
            emplacement_occupied: index,
            slab:                 index,
            rubble:               index,
            slab_destroyed:       index,
            door:                 index,
            stair_up:             index,
            stair_down:           index,
            ladder:               index,
            door_ns:              index,
            door_ew:              index,
            stair_ns_up:          index,
            stair_ns_down:        index,
            stair_ew_up:          index,
            stair_ew_down:        index,
        }
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
    /// `TileRoles` / `TopDownAtlases` insert, leaving the witness reading `false` (nothing
    /// changed) before the event under test — so a `true` afterwards is the redrive's poke.
    fn settle(app: &mut App) {
        app.update();
        app.update();
    }

    /// C4/C8(b): a `Modified` for the TERRAIN sheet image triggers the terrain redraw — the
    /// redrive calls `set_changed()` on `TileRoles`, which the downstream witness sees as
    /// `is_changed()`. Driven through the REAL registered system via `app.update()`.
    ///
    /// Pin-discriminating: dropping the terrain `set_changed()` leaves the witness `false`;
    /// a wrong id map would not fire for the terrain id.
    #[test]
    fn terrain_image_modified_triggers_the_terrain_redraw() {
        let mut app = app();
        let (terrain, _characters) = insert_atlases(&mut app);
        app.world_mut().insert_resource(uniform_roles());
        settle(&mut app);
        assert!(
            !app.world().resource::<RolesChangedWitness>().changed,
            "precondition: after settling, TileRoles must NOT be is_changed",
        );

        inject_modified(&mut app, &terrain);
        app.update();

        assert!(
            app.world().resource::<RolesChangedWitness>().changed,
            "a Modified for the TERRAIN sheet image must set_changed() TileRoles \
             (forcing the terrain re-render)",
        );
    }

    /// C10(e): a `Modified` for a LOADED NON-terrain sheet image (characters) is handled but
    /// does NOT trigger the terrain redraw — the redrive logs the reload yet leaves `TileRoles`
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
        app.world_mut().insert_resource(uniform_roles());
        settle(&mut app);
        assert!(
            !app.world().resource::<RolesChangedWitness>().changed,
            "precondition: after settling, TileRoles must NOT be is_changed",
        );

        inject_modified(&mut app, &characters);
        app.update();

        assert!(
            !app.world().resource::<RolesChangedWitness>().changed,
            "a Modified for a LOADED NON-terrain sheet (characters) must be handled WITHOUT \
             marking TileRoles changed — only the terrain sheet pokes the terrain redraw",
        );
    }

    /// C4/C8(b): a `Modified` for a NON-sheet (unrelated) image id does NOT trigger the
    /// terrain redraw — the id maps to no sheet, so `TileRoles` stays unchanged.
    ///
    /// Pin-discriminating: a redrive that poked on ANY image event (not just a loaded sheet's)
    /// would flip the witness `true` here.
    #[test]
    fn unrelated_image_modified_does_not_trigger_the_terrain_redraw() {
        let mut app = app();
        let (_terrain, _characters) = insert_atlases(&mut app);
        app.world_mut().insert_resource(uniform_roles());
        // An image handle that is NOT registered in TopDownAtlases (a portrait node, a one-off
        // texture, …).
        let unrelated = add_image(&mut app);
        settle(&mut app);

        inject_modified(&mut app, &unrelated);
        app.update();

        assert!(
            !app.world().resource::<RolesChangedWitness>().changed,
            "a Modified for an id that is NOT a loaded sheet must NOT touch TileRoles",
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
        app.world_mut().insert_resource(uniform_roles());
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
}
