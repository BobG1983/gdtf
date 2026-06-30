//! The map-editor **left tile palette** + the **bottom-right stat region** (GTW-422; swept onto
//! the UUID-keyed terrain model in GTW-495).
//!
//! Populates the GTW-417 [`LeftPaletteRegion`](crate::LeftPaletteRegion) scroll list with one
//! WHOLE-TILE row per terrain definition in the ACTIVE theme's palette — each row a tile SPRITE
//! (an [`ImageNode`](bevy::ui::widget::ImageNode) over the terrain sheet at the def's resolved
//! graphic index) beside its display NAME (C1) — and drives the selection / stats flow:
//!
//! - C1 + C4 — [`sync_palette`] lists every terrain of the session's
//!   [`theme`](crate::MapEditorSession::theme), enumerated from the GTW-487
//!   [`UuidThemeRegistry`](gdtf_battle_sim::level::UuidThemeRegistry)'s terrain palette and
//!   resolved against the [`TerrainDefRegistry`](gdtf_battle_sim::terrain::def::TerrainDefRegistry),
//!   and rebuilds the rows when the theme changes.
//! - C2 — [`select_palette_tile`] reads a clicked row, writes its
//!   [`TerrainUuid`](gdtf_battle_sim::terrain::def::TerrainUuid) into
//!   [`MapEditorSession::select_tile`](crate::MapEditorSession::select_tile) and toggles the
//!   [`ActiveButton`](gdtf_ui::ActiveButton) highlight marker onto it.
//! - C3 — [`refresh_stat_region`] fills the [`StatRegion`](crate::StatRegion) with the SELECTED
//!   def's [`sim_kind`](gdtf_battle_sim::terrain::def::TerrainDef::sim_kind) /
//!   [`presenter_kind`](gdtf_battle_sim::terrain::def::TerrainDef::presenter_kind) fields.

use bevy::{
    prelude::*,
    ui::{FlexDirection, widget::ImageNode},
};
use gdtf_battle_presenter::TileRoles;
use gdtf_battle_sim::{
    level::{ThemeUuid, UuidThemeRegistry},
    terrain::def::{TerrainDef, TerrainDefRegistry, TerrainSimKind, TerrainUuid},
};
use gdtf_ui::{ActiveButton, theme::GdtfTheme};

use crate::{
    LeftPaletteRegion, StatRegion, mode::PrefabModeContent, mode_host::mode_host_under_region,
    session::MapEditorSession, terrain_graphics::graphic_key, tile_atlas::TileAtlas,
};

/// One **palette row** — a clickable whole-tile entry (sprite + name) in the left palette
/// (GTW-422 C1/C2; UUID-keyed in GTW-495).
///
/// A [`Component`] carrying the row's [`TerrainUuid`] so the click handler
/// (`select_palette_tile`) knows which terrain to make active. NOT a bare `Uuid` field — the
/// key is the sim's [`TerrainUuid`] domain value (no-bare-types). The row entity is a
/// [`Button`](bevy::ui::widget::Button); the SELECTED row carries the
/// [`ActiveButton`](gdtf_ui::ActiveButton) marker so `gdtf_ui`'s `paint_active_buttons` paints
/// it the theme's active color (never write `BackgroundColor` directly).
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub struct PaletteRow {
    /// The [`TerrainUuid`] this row represents — written into the session on click (C2).
    tile: TerrainUuid,
}

impl PaletteRow {
    /// Build a palette row for a terrain key.
    #[must_use]
    pub const fn new(tile: TerrainUuid) -> Self {
        Self { tile }
    }

    /// The [`TerrainUuid`] this row represents.
    #[must_use]
    pub const fn tile(&self) -> TerrainUuid {
        self.tile
    }
}

/// Marker on the [`StatRegion`](crate::StatRegion)'s text node — the single child whose
/// [`Text`] `refresh_stat_region` rewrites with the selected def's stats (C3).
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub struct StatText;

/// `Update` (in `Editing`): keep the left palette in sync with the session's active theme —
/// the INITIAL populate (C1) AND the theme-change repopulate (C4), in ONE system.
///
/// A single `Update` system because the palette reads the [`TileAtlas`] and the
/// [`MapEditorSession`], BOTH inserted via deferred `Commands` on `OnEnter(Editing)` — so an
/// `OnEnter` populate chained after their inserts would NOT see them (the GTW-421 command-flush
/// race). It (re)builds the rows when the theme the rows were last built for differs from the
/// session's current theme, despawning the existing [`PaletteRow`] entities first, then
/// re-listing from the new theme's terrain palette.
#[expect(
    clippy::too_many_arguments,
    reason = "a Bevy system's params are framework plumbing, not a wide function signature; the \
              palette sync legitimately reads theme + atlas + the theme registry + the terrain \
              registry + the role table + the session + the existing rows + the rebuild tracker"
)]
pub(crate) fn sync_palette(
    mut commands: Commands,
    theme: Res<GdtfTheme>,
    atlas: Option<Res<TileAtlas>>,
    themes: Option<Res<UuidThemeRegistry>>,
    terrain: Option<Res<TerrainDefRegistry>>,
    roles: Option<Res<TileRoles>>,
    session: Option<Res<MapEditorSession>>,
    rows: Query<Entity, With<PaletteRow>>,
    mut last_theme: Local<Option<ThemeUuid>>,
) {
    let (Some(atlas), Some(themes), Some(terrain), Some(roles), Some(session)) =
        (atlas, themes, terrain, roles, session)
    else {
        return;
    };
    let current = session.theme();
    if *last_theme == Some(current) {
        // Already built for this theme — nothing changed.
        return;
    }
    // The active theme's terrain palette (the UUIDs it draws from). An unknown / nil theme has
    // no palette — clear the rows and remember the (empty) build.
    let palette: Vec<TerrainUuid> = themes
        .terrain(&current)
        .map(<[TerrainUuid]>::to_vec)
        .unwrap_or_default();
    for row in &rows {
        commands.entity(row).despawn();
    }
    spawn_palette_rows(&mut commands, &theme, &atlas, &terrain, &roles, &palette);
    *last_theme = Some(current);
}

/// Build + parent one [`PaletteRow`] per terrain in `palette`, deferred-parented under the
/// [`LeftPaletteRegion`]'s [`ScrollListArea`] (the GTW-421 parenting rule). Each row carries its
/// [`TerrainUuid`] (for the click handler) and renders the resolved tile sprite + display name.
fn spawn_palette_rows(
    commands: &mut Commands,
    theme: &GdtfTheme,
    atlas: &TileAtlas,
    terrain: &TerrainDefRegistry,
    roles: &TileRoles,
    palette: &[TerrainUuid],
) {
    let row_bg = *theme.panel.color;
    let text_color = *theme.text.text_color;
    let rows: Vec<Entity> = palette
        .iter()
        .filter_map(|key| terrain.def(key).map(|def| (key, def)))
        .map(|(key, def)| spawn_palette_row(commands, *key, def, atlas, roles, row_bg, text_color))
        .collect();

    commands.queue(move |world: &mut World| {
        // Hang the rows on the left palette's PREFAB-mode content container (under the scrolling
        // viewport) so they top-anchor + scroll AND track the prefab mode's visibility (GTW-474 +
        // the GTW-421 parenting rule).
        let Some(host) = mode_host_under_region::<LeftPaletteRegion, PrefabModeContent>(world)
        else {
            return;
        };
        if let Ok(mut host_entity) = world.get_entity_mut(host) {
            for row in rows {
                host_entity.add_child(row);
            }
        }
    });
}

/// Spawn one whole-tile palette row: a clickable [`Button`](bevy::ui::widget::Button) holding
/// the tile SPRITE (an [`ImageNode`] over the terrain sheet at the def's resolved graphic index)
/// beside its display NAME (C1). Carries the [`PaletteRow`] (its [`TerrainUuid`]) for the click
/// handler. The graphic index is resolved THE WAY THE PRESENTER DOES (via the def's
/// `presenter_kind.graphic_name` against [`TileRoles`]); a def whose role is out of vocabulary
/// gets no sprite (the row still shows its name).
fn spawn_palette_row(
    commands: &mut Commands,
    key: TerrainUuid,
    def: &TerrainDef,
    atlas: &TileAtlas,
    roles: &TileRoles,
    row_bg: Color,
    text_color: Color,
) -> Entity {
    let sprite = match roles.index_for_key(graphic_key(def)) {
        Some(index) => commands
            .spawn((
                ImageNode::from_atlas_image(
                    atlas.image(),
                    TextureAtlas {
                        layout: atlas.layout(),
                        index:  *index,
                    },
                ),
                tile_sprite_node(),
            ))
            .id(),
        None => commands.spawn(tile_sprite_node()).id(),
    };
    let label = commands
        .spawn((
            Text::new((*def.display_name).clone()),
            TextColor(text_color),
            label_node(),
        ))
        .id();
    commands
        .spawn((
            Button,
            PaletteRow::new(key),
            BackgroundColor(row_bg),
            palette_row_node(),
        ))
        .add_children(&[sprite, label])
        .id()
}

/// The press-edge query filter [`select_palette_tile`] reads — a [`PaletteRow`] whose
/// [`Interaction`] changed this frame. A named alias to keep the system signature under
/// clippy's `type_complexity` gate.
type PressedRow = (Changed<Interaction>, With<PaletteRow>);

/// `Update` (in `Editing`): a clicked palette row sets the active paint tile + highlights it
/// (C2).
pub(crate) fn select_palette_tile(
    mut commands: Commands,
    pressed: Query<(Entity, &Interaction), PressedRow>,
    rows: Query<Entity, With<PaletteRow>>,
    session: Option<ResMut<MapEditorSession>>,
    clicked_row: Query<&PaletteRow>,
) {
    let Some(mut session) = session else {
        return;
    };
    // Find the row that was just pressed this frame (the first, if several somehow fired).
    let Some(clicked) = pressed.iter().find_map(|(entity, interaction)| {
        matches!(interaction, Interaction::Pressed).then_some(entity)
    }) else {
        return;
    };
    if let Ok(row) = clicked_row.get(clicked) {
        session.select_tile(row.tile());
    }
    for entity in &rows {
        if entity == clicked {
            commands.entity(entity).insert(ActiveButton);
        } else {
            commands.entity(entity).remove::<ActiveButton>();
        }
    }
}

/// `Update` (in `Editing`): refresh the bottom-right stat region with the selected def's stats
/// (C3).
///
/// Runs when [`MapEditorSession`] is `is_changed()`, resolves the selected [`TerrainUuid`] in the
/// [`TerrainDefRegistry`], and rewrites the [`StatText`] node's [`Text`] with the def's display
/// name + its kind-specific structural stats. With no selection the region shows a placeholder.
pub(crate) fn refresh_stat_region(
    terrain: Option<Res<TerrainDefRegistry>>,
    session: Option<Res<MapEditorSession>>,
    mut stat_text: Query<&mut Text, With<StatText>>,
) {
    let (Some(terrain), Some(session)) = (terrain, session) else {
        return;
    };
    if !session.is_changed() {
        return;
    }
    let Ok(mut text) = stat_text.single_mut() else {
        return;
    };
    let summary = session
        .selected_tile()
        .and_then(|key| terrain.def(&key).map(stat_summary))
        .unwrap_or_else(|| "Select a tile to see its stats.".to_owned());
    *text = Text::new(summary);
}

/// `OnEnter(Editing)`: spawn the [`StatText`] node inside the [`StatRegion`] (C3).
pub(crate) fn spawn_stat_text(mut commands: Commands, theme: Res<GdtfTheme>) {
    let text_color = *theme.text.text_color;
    let text = commands
        .spawn((
            StatText,
            Text::new("Select a tile to see its stats."),
            TextColor(text_color),
            stat_text_node(),
        ))
        .id();
    commands.queue(move |world: &mut World| {
        // The stat text is PREFAB-mode content (the selected-tile stats); hang it on the stat
        // region's PREFAB-mode container so it hides in TERRAIN mode (GTW-474).
        let Some(host) = mode_host_under_region::<StatRegion, PrefabModeContent>(world) else {
            return;
        };
        if let Ok(mut host_entity) = world.get_entity_mut(host) {
            host_entity.add_child(text);
        }
    });
}

/// Render a terrain def's stats into a multi-line human summary (C3) — its display name plus the
/// kind-specific structural fields from its [`sim_kind`](TerrainDef::sim_kind) and the graphic
/// role from its [`presenter_kind`](TerrainDef::presenter_kind), each reusing the sim newtypes'
/// `Deref`'d value.
fn stat_summary(def: &TerrainDef) -> String {
    let name = (*def.display_name).clone();
    let kind = match &def.sim_kind {
        TerrainSimKind::Wall {
            hp,
            armor_protection,
            armor_hardness,
            height_band,
        } => format!(
            "Wall\nHP: {}\nArmor: {}\nHardness: {}\nBand: {height_band:?}",
            **hp, **armor_protection, **armor_hardness,
        ),
        TerrainSimKind::Cover {
            hp,
            armor_protection,
            armor_hardness,
            height_band,
        } => format!(
            "Cover\nHP: {}\nArmor: {}\nHardness: {}\nBand: {height_band:?}",
            **hp, **armor_protection, **armor_hardness,
        ),
        TerrainSimKind::Slab {
            hp,
            armor_protection,
            armor_hardness,
        } => format!(
            "Slab\nHP: {}\nArmor: {}\nHardness: {}",
            **hp, **armor_protection, **armor_hardness,
        ),
    };
    let graphic = (**graphic_key(def)).clone();
    format!("{name}\n{kind}\nGraphic: {graphic}")
}

/// One palette row's [`Node`]: a full-width flex ROW (sprite left, name right). Relative units
/// (no fixed `Px`) per the responsive-UI rule.
fn palette_row_node() -> Node {
    Node {
        width: Val::Percent(100.0),
        flex_direction: FlexDirection::Row,
        align_items: AlignItems::Center,
        column_gap: Val::Vw(0.6),
        padding: UiRect::all(Val::Vh(0.6)),
        ..default()
    }
}

/// The tile-sprite [`Node`] inside a palette row: a small square, sized in viewport-relative
/// units so it scales with the window.
fn tile_sprite_node() -> Node {
    Node {
        width: Val::Vw(2.5),
        height: Val::Vw(2.5),
        ..default()
    }
}

/// The row-label [`Node`]: grows to take the remaining row width beside the sprite.
fn label_node() -> Node {
    Node {
        flex_grow: 1.0,
        ..default()
    }
}

/// The [`StatText`] node's [`Node`]: full-width with a little padding. Relative units.
fn stat_text_node() -> Node {
    Node {
        width: Val::Percent(100.0),
        padding: UiRect::all(Val::Vh(1.0)),
        ..default()
    }
}
