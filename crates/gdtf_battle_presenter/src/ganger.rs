//! The ganger draw (GTW-48 S5 / GTW-219): each sim ganger drawn as a 16x16 top-down
//! SPRITE, kept current by Bevy change detection.
//!
//! This module reads the sim's per-field ganger components — [`Position`], [`Faction`],
//! [`Facing`], [`Stance`], [`Aiming`], [`LifeState`] — and mirrors each ganger as one
//! 16x16 character [`Sprite`] from the role-separated character sheet
//! ([`SheetRole::Characters`], `assets/tiles/alt_tileset_characters.png`). It is the
//! ganger arm of the change-driven sim->view mirror (ADR-0001): the presenter READS the
//! sim and renders it, the sim never reads the presenter.
//!
//! WHICH actor tile a faction draws is DATA-DRIVEN — a per-faction base actor
//! [`TileIndex`] authored in `assets/tiles/character_roles.ron` and resolved into the
//! [`CharacterRoles`] resource. HOW the sim's 8 facings collapse to the sheet's 4
//! sprite frames is a pure, documented mapping ([`facing_frame`]) — the sheet ships 4
//! frames per actor, not 8, so 8-direction sprite generation is not viable. The drawn
//! atlas index is `faction_base + facing_frame`, both terms read STRUCTURALLY (the base
//! from the data table, the offset from the map), never a hardcoded literal.
//!
//! # Draw lifecycle
//!
//! Driven entirely by change detection over the sim's ganger components (registered by
//! [`TopDownRendererPlugin`](crate::TopDownRendererPlugin) in the S4-defined
//! [`PresenterSystems::Draw`] set, ordered `.after(SimSystems::Simulate)`):
//!
//! - [`spawn_ganger_sprites`] — [`Added<Position>`]: spawns one presenter [`Sprite`] for
//!   a ganger on the [`ActiveLevel`], recording its `sim Entity -> presenter Entity` in
//!   the [`GangerSprites`] map and tagging it with the [`GangerSprite`] marker.
//! - [`move_ganger_sprites`] — [`Changed<Position>`] (excluding the spawn): moves the
//!   existing presenter sprite's [`Transform`] (it does NOT respawn) and shows/hides it
//!   by whether the new `(cell, level)` is on the active level.
//! - [`reframe_ganger_sprites`] — [`Changed<Facing>`] / [`Changed<Stance>`] /
//!   [`Changed<Aiming>`]: recomputes the atlas index (facing reframe) and re-tints the
//!   sprite (the stance / aiming delta) in place.
//! - [`update_ganger_life_state`] — [`Changed<LifeState>`]: tints/reframes a `Downed`
//!   ganger and despawns a `Dead` one (dropping its [`GangerSprites`] entry).
//! - [`despawn_removed_ganger_sprites`] — [`RemovedComponents<Position>`]: despawns the
//!   mapped presenter sprite and drops its map entry.
//! - [`apply_active_level_filter`] — on an [`ActiveLevel`] change: hides off-level ganger
//!   sprites and shows on-level ones (the SAME active-level filter the S4 static draw
//!   uses — compare the ganger's `Position` `z` against `**ActiveLevel`).
//!
//! It draws ONLY ganger sprites — never the S4 [`TerrainSprite`](crate::TerrainSprite),
//! never the S2 [`WorldCamera`](crate::WorldCamera) — and adds ZERO sim setup/teardown
//! plumbing (S5 only READS the running battle's results).

use bevy::{
    camera::visibility::RenderLayers, ecs::lifecycle::RemovedComponents,
    platform::collections::HashMap, prelude::*,
};
use gdtf_assets::RonAsset;
use gdtf_battle_sim::{
    Aiming, Cell, Direction, Facing, Faction, Level, LifeState, Position, Stance, StanceKind,
};
use serde::Deserialize;

use crate::{ActiveLevel, CELL_PX, SheetRole, TileIndex, TopDownAtlases, cell_to_world};

/// The DATA-DRIVEN per-faction base actor table — each faction (gang) -> its base
/// [`TileIndex`] into the character sheet.
///
/// Loaded from the loose `assets/tiles/character_roles.ron` through the generic
/// [`RonAsset<T>`](gdtf_assets::RonAsset) loader and resolved into a presenter-owned
/// [`CharacterRoles`] resource before battle time (see [`resolve_character_roles`]),
/// exactly mirroring the S4 `tile_roles.ron` / `load_tile_roles` / `resolve_tile_roles`
/// chain. Each faction's actor is a contiguous run of 4 cells in the sheet; the drawn
/// index is `base + facing_frame` ([`facing_frame`]). Every index is data the engineer
/// eyeballs against the sheet and may adjust — nothing about the index choices is
/// hardcoded in Rust; this struct only names the FACTIONS.
///
/// Derives [`Resource`] (the resolved runtime form), [`Deserialize`] (the authored
/// `.ron` shape), and [`TypePath`] (the bound [`RonAsset<CharacterRoles>`] requires of
/// its payload). At minimum faction 0 and faction 1 map to two visibly distinct actor
/// base tiles; a faction with no authored base falls back to [`Self::faction_0`] (the
/// only design factions today are 0 and 1, the two gangs).
#[derive(Resource, Debug, Clone, PartialEq, Eq, Deserialize, TypePath)]
pub struct CharacterRoles {
    /// Faction (gang) 0's base actor tile index — the first gang's 4-frame actor.
    pub faction_0: TileIndex,
    /// Faction (gang) 1's base actor tile index — the opposing gang's 4-frame actor,
    /// authored to a visibly distinct actor from [`Self::faction_0`].
    pub faction_1: TileIndex,
}

impl CharacterRoles {
    /// The base actor [`TileIndex`] for `faction`, read from this resource.
    ///
    /// Faction 0 -> [`Self::faction_0`], faction 1 -> [`Self::faction_1`]. Any other
    /// gang index (none exist in the current two-gang design) falls back to
    /// [`Self::faction_0`] rather than panicking on an out-of-table faction.
    ///
    /// Not `const`: reading the gang index derefs [`Faction`]'s derived [`Deref`], which
    /// is not a `const` impl (the `f32`-newtype / `Deref`-blocks-const idiom).
    #[must_use]
    pub fn base_for(&self, faction: Faction) -> TileIndex {
        match *faction {
            1 => self.faction_1,
            _ => self.faction_0,
        }
    }
}

/// The path of the loose character-role RON, relative to the asset source root.
const CHARACTER_ROLES_RON_PATH: &str = "tiles/character_roles.ron";

/// The in-flight handle to the character-role RON, held until it resolves into
/// [`CharacterRoles`].
///
/// A named newtype over the bevy [`Handle`] (no-bare-types: a bare handle carries no
/// domain meaning; this name says "the character-role table being loaded"). Inserted by
/// [`load_character_roles`] and read by [`resolve_character_roles`] — the S4
/// `TileRolesHandle` precedent for the character table.
#[derive(Resource, Deref, Debug, Clone)]
pub struct CharacterRolesHandle(pub Handle<RonAsset<CharacterRoles>>);

/// `Startup`: kick off the `character_roles.ron` load, storing its typed handle.
///
/// Loads `tiles/character_roles.ron` as a
/// [`RonAsset<CharacterRoles>`](gdtf_assets::RonAsset) through the generic GTW-136 loader
/// and inserts the [`CharacterRolesHandle`] the [`resolve_character_roles`] poll system
/// reads — the S4 [`load_tile_roles`](crate::load_tile_roles) precedent. Takes
/// `Option<Res<AssetServer>>` so a `MinimalPlugins` headless app with no [`AssetServer`]
/// no-ops rather than panicking (`bevy-traps.md` #1); under `DefaultPlugins` (the app +
/// the `AssetServer` harness) the load fires for real and the resolve runs.
///
/// Param-only (`bevy-traps.md` #7): [`Commands`] for the handle insert, the optional
/// [`Res<AssetServer>`] for the load.
pub fn load_character_roles(mut commands: Commands, asset_server: Option<Res<AssetServer>>) {
    let Some(asset_server) = asset_server else {
        return;
    };
    let handle = asset_server.load::<RonAsset<CharacterRoles>>(CHARACTER_ROLES_RON_PATH);
    commands.insert_resource(CharacterRolesHandle(handle));
}

/// `Update` (gated until [`CharacterRoles`] is resolved): resolve the loaded RON into
/// the presenter-owned [`CharacterRoles`] resource.
///
/// Once the [`RonAsset<CharacterRoles>`](gdtf_assets::RonAsset) has settled into
/// `Assets<RonAsset<CharacterRoles>>` (a transient one-frame "loaded but not yet in the
/// collection" state simply leaves it un-inserted this pass — retried next frame), it
/// clones the deserialized [`CharacterRoles`] out and inserts it as the resident
/// resource so it is present before the first ganger draws. Run only while
/// [`CharacterRolesHandle`] exists AND [`CharacterRoles`] does NOT (the plugin's
/// run-condition), so it inserts once — the S4 [`resolve_tile_roles`](crate::resolve_tile_roles)
/// precedent.
///
/// Param-only (`bevy-traps.md` #7): [`Commands`] for the insert,
/// [`Res<CharacterRolesHandle>`] for the handle,
/// [`Res<Assets<RonAsset<CharacterRoles>>>`] for the loaded asset.
pub fn resolve_character_roles(
    mut commands: Commands,
    handle: Res<CharacterRolesHandle>,
    roles_assets: Res<Assets<RonAsset<CharacterRoles>>>,
) {
    let Some(loaded) = roles_assets.get(&**handle) else {
        // Loaded-but-not-yet-in-collection (or still loading) — retry next frame; the
        // run-condition keeps this system alive until CharacterRoles is resolved.
        return;
    };
    commands.insert_resource((**loaded).clone());
}

/// The presenter-owned map linking each sim ganger [`Entity`] to its presenter sprite
/// [`Entity`].
///
/// A NAMED newtype [`Resource`] over a [`HashMap`] (no-bare-types: a domain map, not a
/// bare collection field; the inner [`Entity`] keys/values are the framework
/// carve-out). `init_resource`-d by [`TopDownRendererPlugin`](crate::TopDownRendererPlugin)
/// so it is present for the whole battle span — the spawn system records into it, the
/// move / reframe / death / removal / level-filter systems look up through it.
#[derive(Resource, Default, Debug)]
pub struct GangerSprites {
    /// Each `sim ganger Entity -> presenter sprite Entity` link. The framework
    /// `Entity` keys/values are the no-bare-types carve-out (a Bevy-owned identity).
    map: HashMap<Entity, Entity>,
}

impl GangerSprites {
    /// Record (or overwrite) the presenter sprite mirroring `sim` ganger.
    fn insert(&mut self, sim: Entity, sprite: Entity) {
        self.map.insert(sim, sprite);
    }

    /// The presenter sprite mirroring `sim` ganger, if one is mapped.
    #[must_use]
    pub fn sprite_for(&self, sim: Entity) -> Option<Entity> {
        self.map.get(&sim).copied()
    }

    /// Drop the mapping for `sim` ganger, returning the presenter sprite it had (if
    /// any) so the caller can despawn it.
    fn remove(&mut self, sim: Entity) -> Option<Entity> {
        self.map.remove(&sim)
    }

    /// Whether a presenter sprite is currently mapped for `sim` ganger.
    #[must_use]
    pub fn contains(&self, sim: Entity) -> bool {
        self.map.contains_key(&sim)
    }
}

/// Marker tagging every ganger sprite this slice spawns, carrying the sim [`Entity`] it
/// mirrors.
///
/// So a redraw / reframe / despawn finds exactly the ganger sprites — and ONLY them,
/// never the S4 [`TerrainSprite`](crate::TerrainSprite), never the S2
/// [`WorldCamera`](crate::WorldCamera). The inner [`Entity`] is the framework carve-out
/// (a Bevy-owned identity, not a domain value); the marker tags only the presenter
/// sprites this slice spawns.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GangerSprite {
    /// The sim ganger [`Entity`] this presenter sprite mirrors.
    pub entity: Entity,
}

/// The 0..=3 within-actor column offset selecting which of the sheet's 4 frames a
/// facing draws.
///
/// A named newtype over `usize` (no-bare-types: a frame offset is a domain value, not a
/// bare `usize`), [`Deref`]ing to it. Added to a faction's base [`TileIndex`] to form
/// the drawn atlas index (`faction_base + facing_frame`). The four frames are the
/// sheet's per-actor order: `0` LEFT (W), `1` DOWN (S), `2` UP (N), `3` RIGHT (E).
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FacingFrame(usize);

impl FacingFrame {
    /// The `col+0` LEFT (W) frame.
    pub const LEFT: Self = Self(0);
    /// The `col+1` DOWN (S) frame.
    pub const DOWN: Self = Self(1);
    /// The `col+2` UP (N) frame.
    pub const UP: Self = Self(2);
    /// The `col+3` RIGHT (E) frame.
    pub const RIGHT: Self = Self(3);
}

/// The pure 8->4 facing-frame map: which of the sheet's 4 frames a [`Direction`] draws.
///
/// The sheet ships 4 frames per actor, not 8, so the 8 sim facings collapse to 4 sprite
/// frames by nearest cardinal with a vertical bias (keeping the face legible). The map
/// is EXACTLY (the S5 contract):
///
/// - [`North`](Direction::North) -> [`UP`](FacingFrame::UP),
///   [`NorthEast`](Direction::NorthEast) -> [`UP`](FacingFrame::UP),
///   [`NorthWest`](Direction::NorthWest) -> [`UP`](FacingFrame::UP)
/// - [`East`](Direction::East) -> [`RIGHT`](FacingFrame::RIGHT)
/// - [`SouthEast`](Direction::SouthEast) -> [`DOWN`](FacingFrame::DOWN),
///   [`South`](Direction::South) -> [`DOWN`](FacingFrame::DOWN),
///   [`SouthWest`](Direction::SouthWest) -> [`DOWN`](FacingFrame::DOWN)
/// - [`West`](Direction::West) -> [`LEFT`](FacingFrame::LEFT)
///
/// A pure `const fn` (NOT data-driven): it is a fixed property of the 4-frame sheet, and
/// encoding it in code keeps it exhaustively unit-testable.
#[must_use]
pub const fn facing_frame(direction: Direction) -> FacingFrame {
    match direction {
        Direction::North | Direction::NorthEast | Direction::NorthWest => FacingFrame::UP,
        Direction::East => FacingFrame::RIGHT,
        Direction::SouthEast | Direction::South | Direction::SouthWest => FacingFrame::DOWN,
        Direction::West => FacingFrame::LEFT,
    }
}

/// The drawn atlas index for a ganger: its faction's base actor tile plus its facing
/// frame.
///
/// `faction_base + facing_frame` — the base read STRUCTURALLY from [`CharacterRoles`]
/// (the data table) and the offset from the [`facing_frame`] map, never a hardcoded
/// literal. Returned as a flat `usize` for [`TextureAtlas::index`].
#[must_use]
fn atlas_index(roles: &CharacterRoles, faction: Faction, facing: Facing) -> usize {
    *roles.base_for(faction) + *facing_frame(*facing)
}

/// The faction (gang) tint applied to a ganger sprite so the two gangs read as two
/// colours at a glance — the "faction-coloured" signal layered on top of the distinct
/// per-faction actor tile.
///
/// Faction 0 draws a cool blue tint, faction 1 a warm red tint (any other gang, none in
/// the two-gang design, reuses faction 0's). [`Color`] is a framework type, so the tint
/// itself is framework plumbing; this fn is the per-faction CHOICE. A [`Downed`]
/// ganger overrides this with [`downed_tint`].
#[must_use]
fn faction_tint(faction: Faction) -> Color {
    match *faction {
        1 => Color::srgb(1.0, 0.55, 0.5),
        _ => Color::srgb(0.55, 0.7, 1.0),
    }
}

/// The grey-out tint a [`LifeState::Downed`] ganger draws with — a desaturated, dimmed
/// overlay so a downed body reads as out of the fight while still on the field.
///
/// The documented Downed delta the AC asserts: a distinct darker / desaturated tint
/// from the live faction tint. [`Color`] is framework plumbing; this fn is the CHOICE.
#[must_use]
const fn downed_tint() -> Color {
    Color::srgb(0.4, 0.4, 0.45)
}

/// The tint a ganger sprite draws with given its faction and life state.
///
/// A live ([`LifeState::Alive`]) ganger draws its [`faction_tint`]; a [`Downed`] ganger
/// draws the [`downed_tint`] (a [`Dead`](LifeState::Dead) ganger has no sprite — it is
/// despawned). The aiming delta is layered separately in [`reframe_ganger_sprites`].
#[must_use]
fn ganger_tint(faction: Faction, life: LifeState) -> Color {
    match life {
        LifeState::Downed => downed_tint(),
        // Dead has no sprite (despawned); treat it as the live tint for completeness.
        LifeState::Alive | LifeState::Dead => faction_tint(faction),
    }
}

/// The world `(cell, level)` a ganger's [`Position`] projects to — reconstruct the typed
/// [`Cell`] / [`Level`] from the position's `IVec3` components (the S4 idiom), since
/// [`Position`] Derefs to [`CellLevel`](gdtf_battle_sim::CellLevel) Derefs to `IVec3`.
///
/// `pos.z` is a storey index in `0..MAX_LEVELS`; clamping the (impossible-in-practice)
/// negative / over-`u8` case keeps the reconstruction panic-free.
fn cell_and_level(pos: &Position) -> (Cell, Level) {
    let cell = Cell::new(pos.x, pos.y);
    let storey = u8::try_from(pos.z).unwrap_or(0);
    (cell, Level::new(storey))
}

/// Whether a ganger at `pos` is on the presenter's `active` storey.
///
/// The SAME active-level filter the S4 static draw uses: compare the ganger's
/// `Position` `z` against the active [`Level`] (`bevy-traps.md` consistency with the
/// terrain filter). `active` is the dereferenced [`ActiveLevel`]'s inner [`Level`].
fn on_active_level(pos: &Position, active: Level) -> bool {
    pos.z == i32::from(*active)
}

/// Build one ganger [`Sprite`] on the character sheet at `index`, tinted `tint`, via the
/// S3 recipe.
///
/// `Sprite::from_atlas_image(chars.image, TextureAtlas { layout, index })` with
/// `custom_size = Some(Vec2::splat(CELL_PX))` (the documented S3 sizing recipe) and the
/// `tint` applied to `Sprite.color`. Returns [`None`] if the character sheet was not
/// loaded (so the caller skips the spawn rather than panic).
fn ganger_sprite(index: usize, tint: Color, atlases: &TopDownAtlases) -> Option<Sprite> {
    let chars = atlases.role(SheetRole::Characters)?;
    let mut sprite = Sprite::from_atlas_image(
        chars.image.clone(),
        TextureAtlas {
            layout: chars.layout.clone(),
            index,
        },
    );
    sprite.custom_size = Some(Vec2::splat(CELL_PX));
    sprite.color = tint;
    Some(sprite)
}

/// `Update` (`PresenterSystems::Draw`): spawn one presenter sprite per newly-added
/// ganger on the active level.
///
/// For every ganger whose [`Position`] was [`Added`] this update AND is on the
/// [`ActiveLevel`], build a [`Sprite`] (atlas index `faction_base + facing_frame`, the
/// faction tint) at [`cell_to_world`](crate::cell_to_world), on the
/// [`WORLD_RENDER_LAYER`](crate::WORLD_RENDER_LAYER), with the [`GangerSprite`] marker;
/// record `sim Entity -> presenter Entity` in [`GangerSprites`]. Off-`ActiveLevel`
/// gangers are spawned HIDDEN (a [`Visibility::Hidden`] sprite is recorded too) so a
/// later [`apply_active_level_filter`] can show it without a respawn — the level filter
/// is uniform across spawn / move / level-change.
///
/// Param-only (`bevy-traps.md` #7): [`Commands`], [`ResMut<GangerSprites>`], the read
/// resources, and the [`Added<Position>`] ganger query.
pub fn spawn_ganger_sprites(
    mut commands: Commands,
    mut sprites: ResMut<GangerSprites>,
    roles: Res<CharacterRoles>,
    atlases: Res<TopDownAtlases>,
    active: Res<ActiveLevel>,
    added: Query<(Entity, &Position, &Faction, &Facing, &LifeState), Added<Position>>,
) {
    for (entity, pos, faction, facing, life) in &added {
        // A Dead ganger added directly (no live frame) draws no sprite.
        if matches!(life, LifeState::Dead) {
            continue;
        }
        let index = atlas_index(&roles, *faction, *facing);
        let tint = ganger_tint(*faction, *life);
        let Some(sprite) = ganger_sprite(index, tint, &atlases) else {
            continue;
        };
        let (cell, level) = cell_and_level(pos);
        let visibility = if on_active_level(pos, **active) {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        let presenter = commands
            .spawn((
                sprite,
                Transform::from_translation(cell_to_world(cell, level)),
                visibility,
                RenderLayers::layer(crate::WORLD_RENDER_LAYER),
                GangerSprite { entity },
            ))
            .id();
        sprites.insert(entity, presenter);
    }
}

/// `Update` (`PresenterSystems::Draw`, `.after(spawn_ganger_sprites)`): move (do NOT
/// respawn) the presenter sprite of a ganger whose [`Position`] changed.
///
/// For every ganger whose [`Position`] is [`Changed`], look the presenter sprite up
/// through [`GangerSprites`] and move its [`Transform`] to the new
/// [`cell_to_world`](crate::cell_to_world), updating its [`Visibility`] by whether the
/// new `(cell, level)` is on the [`ActiveLevel`]. It does NOT spawn a second sprite: it
/// is idempotent via the map (a just-`Added` ganger handled by [`spawn_ganger_sprites`]
/// this same update is already mapped — `.after(spawn_ganger_sprites)` guarantees the
/// entry exists — so this only re-sets the same transform; a not-yet-mapped ganger is
/// skipped). The contract's "idempotent via the map" move path.
///
/// Param-only (`bevy-traps.md` #7): [`Res<GangerSprites>`], [`Res<ActiveLevel>`], the
/// moved-ganger query, and the presenter-sprite [`Transform`] / [`Visibility`] query.
pub fn move_ganger_sprites(
    sprites: Res<GangerSprites>,
    active: Res<ActiveLevel>,
    moved: Query<(Entity, &Position), Changed<Position>>,
    mut presenters: Query<(&mut Transform, &mut Visibility), With<GangerSprite>>,
) {
    for (entity, pos) in &moved {
        let Some(presenter) = sprites.sprite_for(entity) else {
            continue;
        };
        let Ok((mut transform, mut visibility)) = presenters.get_mut(presenter) else {
            continue;
        };
        let (cell, level) = cell_and_level(pos);
        transform.translation = cell_to_world(cell, level);
        *visibility = if on_active_level(pos, **active) {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
}

/// The read-only ganger fields the reframe/re-tint read — the [`QueryData`] tuple,
/// factored out to keep the [`reframe_ganger_sprites`] query under the
/// `type_complexity` clippy gate.
///
/// [`QueryData`]: bevy::ecs::query::QueryData
type ReframeData = (
    Entity,
    &'static Faction,
    &'static Facing,
    &'static Stance,
    &'static Aiming,
    &'static LifeState,
);

/// The "any of facing / stance / aiming changed" [`QueryFilter`] driving the
/// reframe/re-tint, factored out for the same `type_complexity` reason as
/// [`ReframeData`].
///
/// [`QueryFilter`]: bevy::ecs::query::QueryFilter
type ReframeChanged = Or<(Changed<Facing>, Changed<Stance>, Changed<Aiming>)>;

/// `Update` (`PresenterSystems::Draw`): reframe / re-tint a ganger sprite whose
/// [`Facing`], [`Stance`], or [`Aiming`] changed.
///
/// For every ganger whose [`Facing`] / [`Stance`] / [`Aiming`] is [`Changed`], look the
/// presenter sprite up through [`GangerSprites`] and recompute its texture-atlas index
/// (facing reframe via the 8->4 map) and re-tint it (the stance / aiming delta) in
/// place. The reframe always recomputes from the CURRENT facing; the aiming delta
/// brightens the sprite (an aimed ganger reads "ready"); the stance delta dims a prone
/// ganger (a flattened silhouette). A [`Dead`](LifeState::Dead) ganger's sprite is
/// already despawned, so its lookup misses and is skipped.
///
/// Param-only (`bevy-traps.md` #7): [`Res<GangerSprites>`], the changed-state ganger
/// query, and the presenter-sprite [`Sprite`] query.
pub fn reframe_ganger_sprites(
    sprites: Res<GangerSprites>,
    roles: Res<CharacterRoles>,
    changed: Query<ReframeData, ReframeChanged>,
    mut presenters: Query<&mut Sprite, With<GangerSprite>>,
) {
    for (entity, faction, facing, stance, aiming, life) in &changed {
        let Some(presenter) = sprites.sprite_for(entity) else {
            continue;
        };
        let Ok(mut sprite) = presenters.get_mut(presenter) else {
            continue;
        };
        // Reframe to the facing-correct frame.
        if let Some(atlas) = sprite.texture_atlas.as_mut() {
            atlas.index = atlas_index(&roles, *faction, *facing);
        }
        // Re-tint: the faction/life base, modulated by the stance + aiming delta.
        sprite.color = stance_aiming_tint(*faction, *life, *stance, *aiming);
    }
}

/// `Update` (`PresenterSystems::Draw`): apply a [`Changed<LifeState>`] to a ganger
/// sprite.
///
/// A [`Downed`](LifeState::Downed) ganger's sprite is re-tinted to the [`downed_tint`]
/// (greyed out, out of the fight); a [`Dead`](LifeState::Dead) ganger's presenter sprite
/// is DESPAWNED and its [`GangerSprites`] entry dropped (the contract's chosen
/// death-delta — despawn, not a corpse tile). An [`Alive`](LifeState::Alive) transition
/// (a revive) restores the live tint.
///
/// Param-only (`bevy-traps.md` #7): [`Commands`], [`ResMut<GangerSprites>`], the
/// changed-life query, and the presenter-sprite [`Sprite`] query.
pub fn update_ganger_life_state(
    mut commands: Commands,
    mut sprites: ResMut<GangerSprites>,
    changed: Query<(Entity, &Faction, &LifeState), Changed<LifeState>>,
    mut presenters: Query<&mut Sprite, With<GangerSprite>>,
) {
    for (entity, faction, life) in &changed {
        match life {
            LifeState::Dead => {
                // Despawn the presenter sprite and drop its map entry.
                if let Some(presenter) = sprites.remove(entity) {
                    commands.entity(presenter).despawn();
                }
            }
            LifeState::Downed | LifeState::Alive => {
                let Some(presenter) = sprites.sprite_for(entity) else {
                    continue;
                };
                let Ok(mut sprite) = presenters.get_mut(presenter) else {
                    continue;
                };
                sprite.color = ganger_tint(*faction, *life);
            }
        }
    }
}

/// `Update` (`PresenterSystems::Draw`): despawn the presenter sprite of a ganger whose
/// [`Position`] was REMOVED.
///
/// Drains [`RemovedComponents<Position>`] (from `bevy::ecs::removal_detection`); for each
/// removed sim entity it despawns the mapped presenter sprite and drops its
/// [`GangerSprites`] entry. A ganger losing its [`Position`] (e.g. removed from the
/// battle) leaves no orphan sprite behind.
///
/// Param-only (`bevy-traps.md` #7): [`Commands`], [`ResMut<GangerSprites>`],
/// [`RemovedComponents<Position>`].
pub fn despawn_removed_ganger_sprites(
    mut commands: Commands,
    mut sprites: ResMut<GangerSprites>,
    mut removed: RemovedComponents<Position>,
) {
    for entity in removed.read() {
        if let Some(presenter) = sprites.remove(entity) {
            commands.entity(presenter).despawn();
        }
    }
}

/// `Update` (`PresenterSystems::Draw`, runs only on an [`ActiveLevel`] change): show the
/// ganger sprites on the new active level, hide the rest.
///
/// On an [`ActiveLevel`] change ([`ActiveLevel::is_changed`]) it walks every live ganger
/// and sets its mapped presenter sprite's [`Visibility`] by whether the ganger's
/// `Position` is on the new active level (the SAME `pos.z == **ActiveLevel` filter the
/// S4 static draw uses). Off-level sprites are HIDDEN (not despawned — the move / reframe
/// systems keep them current), on-level sprites are SHOWN. It is gated to only run when
/// the resource changed so it does no per-frame work.
///
/// Param-only (`bevy-traps.md` #7): [`Res<GangerSprites>`], [`Res<ActiveLevel>`], the
/// ganger [`Position`] query, and the presenter-sprite [`Visibility`] query.
pub fn apply_active_level_filter(
    sprites: Res<GangerSprites>,
    active: Res<ActiveLevel>,
    gangers: Query<(Entity, &Position)>,
    mut presenters: Query<&mut Visibility, With<GangerSprite>>,
) {
    if !active.is_changed() {
        return;
    }
    for (entity, pos) in &gangers {
        let Some(presenter) = sprites.sprite_for(entity) else {
            continue;
        };
        let Ok(mut visibility) = presenters.get_mut(presenter) else {
            continue;
        };
        *visibility = if on_active_level(pos, **active) {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
}

/// The tint a ganger sprite draws with given its faction, life state, stance, and aiming
/// flag — the combined re-tint [`reframe_ganger_sprites`] applies.
///
/// Starts from [`ganger_tint`] (the faction / Downed base), then layers the stance +
/// aiming deltas: a [`Prone`](gdtf_battle_sim::StanceKind::Prone) ganger dims (a
/// flattened, low silhouette), and an aiming ganger brightens (reads "ready to fire").
/// The deltas only apply to a live ganger — a Downed body keeps its grey-out, undimmed
/// by stance / aim. [`Color`] is framework plumbing; this fn is the CHOICE the AC
/// asserts.
#[must_use]
fn stance_aiming_tint(faction: Faction, life: LifeState, stance: Stance, aiming: Aiming) -> Color {
    let base = ganger_tint(faction, life);
    // Downed keeps its grey-out — stance / aim do not modulate an out-of-fight body.
    if matches!(life, LifeState::Downed) {
        return base;
    }
    // A prone ganger dims; an aiming ganger brightens. Multiplicative on the linear
    // colour so the faction hue is preserved, only the value shifts.
    let stance_scale = match *stance {
        StanceKind::Prone => 0.7,
        StanceKind::Standing | StanceKind::Crouching => 1.0,
    };
    let aim_scale = if *aiming { 1.2 } else { 1.0 };
    let factor = stance_scale * aim_scale;
    let linear = base.to_linear();
    Color::linear_rgba(
        linear.red * factor,
        linear.green * factor,
        linear.blue * factor,
        linear.alpha,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// AC2 — the 8->4 facing map maps each of the 8 directions to the documented frame,
    /// exhaustively. `North/NorthEast/NorthWest -> UP`, `East -> RIGHT`,
    /// `SouthEast/South/SouthWest -> DOWN`, `West -> LEFT`.
    #[test]
    fn facing_frame_maps_all_eight_directions() {
        assert_eq!(facing_frame(Direction::North), FacingFrame::UP, "N -> UP");
        assert_eq!(
            facing_frame(Direction::NorthEast),
            FacingFrame::UP,
            "NE -> UP",
        );
        assert_eq!(
            facing_frame(Direction::NorthWest),
            FacingFrame::UP,
            "NW -> UP",
        );
        assert_eq!(
            facing_frame(Direction::East),
            FacingFrame::RIGHT,
            "E -> RIGHT",
        );
        assert_eq!(
            facing_frame(Direction::SouthEast),
            FacingFrame::DOWN,
            "SE -> DOWN",
        );
        assert_eq!(
            facing_frame(Direction::South),
            FacingFrame::DOWN,
            "S -> DOWN",
        );
        assert_eq!(
            facing_frame(Direction::SouthWest),
            FacingFrame::DOWN,
            "SW -> DOWN",
        );
        assert_eq!(
            facing_frame(Direction::West),
            FacingFrame::LEFT,
            "W -> LEFT"
        );
    }

    /// The four frame offsets are the documented column offsets `0..=3` in the sheet's
    /// per-actor order (LEFT/DOWN/UP/RIGHT) — the structural pins the atlas-index sum
    /// relies on.
    #[test]
    fn facing_frame_offsets_are_zero_to_three() {
        assert_eq!(*FacingFrame::LEFT, 0, "LEFT is col+0");
        assert_eq!(*FacingFrame::DOWN, 1, "DOWN is col+1");
        assert_eq!(*FacingFrame::UP, 2, "UP is col+2");
        assert_eq!(*FacingFrame::RIGHT, 3, "RIGHT is col+3");
    }

    /// The atlas index is `faction_base + facing_frame`, read structurally from the
    /// table + the map — never a literal. Built from an arbitrary in-test table so it
    /// pins the SUM mechanism, not the shipped data.
    #[test]
    fn atlas_index_is_base_plus_frame() {
        let roles = CharacterRoles {
            faction_0: TileIndex::new(10),
            faction_1: TileIndex::new(20),
        };
        // Faction 0 facing East -> base 10 + RIGHT (3) = 13.
        assert_eq!(
            atlas_index(&roles, Faction::new(0), Facing::new(Direction::East)),
            13,
            "faction 0 + East = faction_0 base + RIGHT offset",
        );
        // Faction 1 facing North -> base 20 + UP (2) = 22.
        assert_eq!(
            atlas_index(&roles, Faction::new(1), Facing::new(Direction::North)),
            22,
            "faction 1 + North = faction_1 base + UP offset",
        );
    }

    /// `base_for` resolves each faction to its authored base, and an out-of-table
    /// faction falls back to faction 0 (no panic).
    #[test]
    fn base_for_resolves_factions_and_falls_back() {
        let roles = CharacterRoles {
            faction_0: TileIndex::new(0),
            faction_1: TileIndex::new(4),
        };
        assert_eq!(roles.base_for(Faction::new(0)), TileIndex::new(0));
        assert_eq!(roles.base_for(Faction::new(1)), TileIndex::new(4));
        // An out-of-table gang index (none exist in the two-gang design) -> faction 0.
        assert_eq!(roles.base_for(Faction::new(7)), TileIndex::new(0));
    }

    /// The two factions resolve to two DISTINCT base indices in the shipped table — the
    /// visibly-distinct-actors guarantee, asserted structurally (not a magnitude pin).
    #[test]
    fn shipped_character_roles_ron_parses_with_distinct_factions() {
        const SHIPPED: &str = include_str!("../../../assets/tiles/character_roles.ron");
        let parsed: Result<CharacterRoles, _> = ron::de::from_str(SHIPPED);
        assert!(
            parsed.is_ok(),
            "shipped character_roles.ron must parse into CharacterRoles, got: {:?}",
            parsed.as_ref().err(),
        );
        let Ok(roles) = parsed else {
            return;
        };
        assert_ne!(
            roles.faction_0, roles.faction_1,
            "the two factions must map to two visibly distinct actor base tiles",
        );
    }

    /// The Downed tint differs from both factions' live tints — the documented Downed
    /// delta is a visible re-tint.
    #[test]
    fn downed_tint_differs_from_live_faction_tints() {
        assert_ne!(
            ganger_tint(Faction::new(0), LifeState::Downed),
            ganger_tint(Faction::new(0), LifeState::Alive),
            "a Downed faction-0 ganger tints differently from a live one",
        );
        assert_ne!(
            ganger_tint(Faction::new(1), LifeState::Downed),
            ganger_tint(Faction::new(1), LifeState::Alive),
            "a Downed faction-1 ganger tints differently from a live one",
        );
    }

    /// The two factions draw two distinct live tints — the "faction-coloured" signal.
    #[test]
    fn faction_tints_are_distinct() {
        assert_ne!(
            faction_tint(Faction::new(0)),
            faction_tint(Faction::new(1)),
            "the two factions must read as two distinct colours",
        );
    }
}
