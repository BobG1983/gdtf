//! The map-editor **left tile palette** + the **bottom-right stat region** (GTW-422).
//!
//! Populates the GTW-417 [`LeftPaletteRegion`](crate::LeftPaletteRegion) scroll list with one
//! WHOLE-TILE row per tile of the ACTIVE theme — each row a tile SPRITE (an
//! [`ImageNode`](bevy::ui::widget::ImageNode) over the terrain sheet at the tile's atlas
//! index) beside its display NAME (C1) — and drives the selection / stats flow:
//!
//! - C1 + C4 — [`sync_palette`] lists every catalog tile of the session's
//!   [`theme`](crate::MapEditorSession::theme), from the GTW-409
//!   [`ThemeCatalogRegistry`](gdtf_battle_sim::level::ThemeCatalogRegistry), and rebuilds the
//!   rows when the theme changes (despawn the old rows, re-list from the new theme's catalog).
//!   The rows hang in the [`LeftPaletteRegion`]'s [`ScrollListArea`](gdtf_ui::ScrollListArea)
//!   (the clipping, scrolling viewport — NOT the grid root frame the marker rides — the GTW-421
//!   parenting rule) so they top-anchor and scroll on overflow.
//! - C2 — [`select_palette_tile`] reads a clicked row
//!   (`Changed<Interaction> == Pressed`), writes its [`TileKey`](gdtf_battle_sim::level::TileKey)
//!   into [`MapEditorSession::select_tile`](crate::MapEditorSession::select_tile) and toggles the
//!   [`ActiveButton`](gdtf_ui::ActiveButton) highlight marker onto it (clearing the other rows').
//! - C3 — [`refresh_stat_region`] fills the [`StatRegion`](crate::StatRegion) with the SELECTED
//!   tile's catalog fields (role / name + cover/slab HP, armor, height band, move cost).

use bevy::{
    prelude::*,
    ui::{FlexDirection, widget::ImageNode},
};
use gdtf_battle_sim::level::{
    CatalogTile, CatalogTileKind, LevelTheme, StructuralStats, ThemeCatalogRegistry,
    ThemeTileCatalog, TileKey,
};
use gdtf_ui::{ActiveButton, ScrollListArea, theme::GdtfTheme};

use crate::{LeftPaletteRegion, StatRegion, session::MapEditorSession, tile_atlas::TileAtlas};

/// One **palette row** — a clickable whole-tile entry (sprite + name) in the left palette
/// (GTW-422 C1/C2).
///
/// A [`Component`] carrying the row's [`TileKey`] so the click handler
/// (`select_palette_tile`) knows which tile to make active. NOT a bare `String` field — the
/// key is the sim's [`TileKey`] domain value (no-bare-types). The row entity is a
/// [`Button`](bevy::ui::widget::Button) (so it `#[require]`s [`Interaction`] and the engine's
/// focus system drives click state); the SELECTED row carries the
/// [`ActiveButton`](gdtf_ui::ActiveButton) marker so `gdtf_ui`'s `paint_active_buttons` paints
/// it the theme's active color (the sanctioned selected-button highlight — never fight the
/// interaction-repaint system by writing `BackgroundColor` directly).
#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub struct PaletteRow {
    /// The catalog [`TileKey`] this row represents — written into the session on click (C2).
    tile: TileKey,
}

impl PaletteRow {
    /// Build a palette row for a catalog tile key.
    #[must_use]
    pub const fn new(tile: TileKey) -> Self {
        Self { tile }
    }

    /// The catalog [`TileKey`] this row represents.
    #[must_use]
    pub const fn tile(&self) -> &TileKey {
        &self.tile
    }
}

/// Marker on the [`StatRegion`](crate::StatRegion)'s text node — the single child whose
/// [`Text`] `refresh_stat_region` rewrites with the selected tile's stats (C3).
///
/// A unit marker (no-bare-types). Lets the refresh find the stat text without depending on the
/// region's tree shape.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub struct StatText;

/// `Update` (in `Editing`): keep the left palette in sync with the session's active theme —
/// the INITIAL populate (C1) AND the theme-change repopulate (C4), in ONE system.
///
/// A single `Update` system (rather than an `OnEnter` populate + an `Update` repopulate)
/// because the palette reads both the [`TileAtlas`] and the [`MapEditorSession`], BOTH inserted
/// via deferred `Commands` on `OnEnter(Editing)` — so an `OnEnter` populate chained after their
/// inserts would NOT see them (no command-flush sync between chained `OnEnter` systems — the
/// GTW-421 `insert_session` race). Running in `Update` sidesteps that: the resources are present
/// the first frame in `Editing`.
///
/// It (re)builds the rows when the theme the rows were last built for differs from the session's
/// current theme — covering BOTH the first build (tracked theme is `None`) and a later switch
/// (the GTW-421 dropdown writes `MapEditorSession::theme`). It despawns the existing
/// [`PaletteRow`] entities first, then re-lists from the active theme's catalog, parenting the
/// new rows under the palette's [`ScrollListArea`] (the clipping, scrolling viewport — the
/// GTW-421 parenting rule). The last-built theme is tracked in a [`Local`] so an unrelated
/// session mutation (grid-size / selected-tile) never triggers a needless rebuild.
pub(crate) fn sync_palette(
    mut commands: Commands,
    theme: Res<GdtfTheme>,
    atlas: Option<Res<TileAtlas>>,
    registry: Option<Res<ThemeCatalogRegistry>>,
    session: Option<Res<MapEditorSession>>,
    rows: Query<Entity, With<PaletteRow>>,
    mut last_theme: Local<Option<LevelTheme>>,
) {
    let (Some(atlas), Some(registry), Some(session)) = (atlas, registry, session) else {
        return;
    };
    let current = session.theme();
    if *last_theme == Some(current) {
        // Already built for this theme — nothing changed.
        return;
    }
    let Some(catalog) = registry.catalog(current) else {
        return;
    };
    for row in &rows {
        commands.entity(row).despawn();
    }
    spawn_palette_rows(&mut commands, &theme, &atlas, catalog);
    *last_theme = Some(current);
}

/// Build + parent one [`PaletteRow`] per tile in `catalog`, deferred-parented under the
/// [`LeftPaletteRegion`]'s [`ScrollListArea`] (the GTW-421 parenting rule). Shared by the
/// `OnEnter` populate (C1) and the theme-change repopulate (C4).
///
/// The tile order is the catalog's iteration order (a `HashMap` — unordered, but the editor
/// lists a SET of tiles, no order contract). Each row carries its [`TileKey`] (for the click
/// handler) and renders the tile sprite + name.
fn spawn_palette_rows(
    commands: &mut Commands,
    theme: &GdtfTheme,
    atlas: &TileAtlas,
    catalog: &ThemeTileCatalog,
) {
    let row_bg = *theme.panel.color;
    let text_color = *theme.text.text_color;
    let rows: Vec<Entity> = catalog
        .tiles()
        .map(|(key, tile)| spawn_palette_row(commands, key, tile, atlas, row_bg, text_color))
        .collect();

    commands.queue(move |world: &mut World| {
        let Some(area) = palette_scroll_area(world) else {
            return;
        };
        if let Ok(mut area_entity) = world.get_entity_mut(area) {
            for row in rows {
                area_entity.add_child(row);
            }
        }
    });
}

/// Find the [`LeftPaletteRegion`]'s [`ScrollListArea`] — the clipping, scrolling viewport child
/// of the region's scroll-list grid root frame (the GTW-421 parenting rule).
///
/// The [`LeftPaletteRegion`] marker rides the scroll-list ROOT FRAME; the rows must hang on the
/// area among its children, not the frame itself. Returns [`None`] if the frame or its area is
/// not yet present (the deferred command runs after the shell spawn applied the scroll list, so
/// the area exists by then).
fn palette_scroll_area(world: &mut World) -> Option<Entity> {
    let frame = world
        .query_filtered::<Entity, With<LeftPaletteRegion>>()
        .iter(world)
        .next()?;
    let children = world.get::<Children>(frame)?;
    children.iter().find(|child| {
        world
            .get_entity(*child)
            .is_ok_and(|entity| entity.contains::<ScrollListArea>())
    })
}

/// Spawn one whole-tile palette row: a clickable [`Button`](bevy::ui::widget::Button) holding
/// the tile SPRITE (an [`ImageNode`] over the terrain sheet at the tile's atlas index) beside
/// its display NAME (C1). Carries the [`PaletteRow`] (its [`TileKey`]) for the click handler.
fn spawn_palette_row(
    commands: &mut Commands,
    key: &TileKey,
    tile: &CatalogTile,
    atlas: &TileAtlas,
    row_bg: Color,
    text_color: Color,
) -> Entity {
    let sprite = commands
        .spawn((
            ImageNode::from_atlas_image(
                atlas.image(),
                TextureAtlas {
                    layout: atlas.layout(),
                    index:  *tile.atlas_index,
                },
            ),
            tile_sprite_node(),
        ))
        .id();
    let label = commands
        .spawn((
            Text::new((*tile.display_name).clone()),
            TextColor(text_color),
            label_node(),
        ))
        .id();
    commands
        .spawn((
            Button,
            PaletteRow::new(key.clone()),
            BackgroundColor(row_bg),
            palette_row_node(),
        ))
        .add_children(&[sprite, label])
        .id()
}

/// The press-edge query filter [`select_palette_tile`] reads — a [`PaletteRow`] whose
/// [`Interaction`] changed this frame. A named alias to keep the system signature under
/// clippy's `type_complexity` gate (the `gdtf_ui` widget-driver precedent).
type PressedRow = (Changed<Interaction>, With<PaletteRow>);

/// `Update` (in `Editing`): a clicked palette row sets the active paint tile + highlights it
/// (C2).
///
/// Reads `Changed<Interaction> == Pressed` on the [`PaletteRow`] buttons (the engine's focus
/// system drives [`Interaction`] under `DefaultPlugins`; headless tests set it directly + run
/// `Update` — the dropdown-widget `press` precedent). On a press it writes the row's [`TileKey`]
/// into the session via [`MapEditorSession::select_tile`] and toggles the
/// [`ActiveButton`](gdtf_ui::ActiveButton) marker — ADD it to the clicked row, REMOVE it from
/// every other row — so `gdtf_ui`'s `paint_active_buttons` paints exactly the selected row the
/// theme's active color (the sanctioned selected-button highlight; the interaction-repaint
/// system skips `ActiveButton` rows, so the highlight is sticky and flicker-free).
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
        session.select_tile(row.tile().clone());
    }
    for entity in &rows {
        if entity == clicked {
            commands.entity(entity).insert(ActiveButton);
        } else {
            commands.entity(entity).remove::<ActiveButton>();
        }
    }
}

/// `Update` (in `Editing`): refresh the bottom-right stat region with the selected tile's
/// catalog stats (C3).
///
/// Runs when [`MapEditorSession`] is `is_changed()` (a selection writes it), resolves the
/// selected [`TileKey`] in the active theme's catalog, and rewrites the [`StatText`] node's
/// [`Text`] with the tile's catalog fields — role + name + the kind-specific gameplay stats
/// (move cost for a floor; HP / armor / height band for a wall / cover / scatter; HP + armor
/// for a slab). Mutates the text IN PLACE (the ui-mutate-not-respawn rule). With no selection
/// the region shows a placeholder prompt.
pub(crate) fn refresh_stat_region(
    registry: Option<Res<ThemeCatalogRegistry>>,
    session: Option<Res<MapEditorSession>>,
    mut stat_text: Query<&mut Text, With<StatText>>,
) {
    let (Some(registry), Some(session)) = (registry, session) else {
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
        .and_then(|key| {
            registry
                .catalog(session.theme())
                .and_then(|catalog| catalog.tile(key).map(|tile| stat_summary(key, tile)))
        })
        .unwrap_or_else(|| "Select a tile to see its stats.".to_owned());
    *text = Text::new(summary);
}

/// `OnEnter(Editing)`: spawn the [`StatText`] node inside the [`StatRegion`] (C3).
///
/// The stat region is a themed panel spawned empty by `spawn_editor_shell`; this hangs one
/// [`StatText`] node under it (deferred, so the panel exists) seeded with the no-selection
/// placeholder. [`refresh_stat_region`] rewrites its [`Text`] on each selection.
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
        let Some(region) = world
            .query_filtered::<Entity, With<StatRegion>>()
            .iter(world)
            .next()
        else {
            return;
        };
        if let Ok(mut region_entity) = world.get_entity_mut(region) {
            region_entity.add_child(text);
        }
    });
}

/// Render a catalog tile's stats into a multi-line human summary (C3) — its role + name plus
/// the kind-specific gameplay fields, each reusing the sim stat newtypes' `Deref`'d value.
fn stat_summary(key: &TileKey, tile: &CatalogTile) -> String {
    let name = (*tile.display_name).clone();
    let kind = match &tile.kind {
        CatalogTileKind::Floor { move_cost } => {
            format!("Floor\nMove cost: {}", **move_cost)
        }
        CatalogTileKind::Wall(stats) => format!("Wall\n{}", structural_lines(stats)),
        CatalogTileKind::Cover(stats) => format!("Cover\n{}", structural_lines(stats)),
        CatalogTileKind::Scatter(stats) => format!("Scatter\n{}", structural_lines(stats)),
        CatalogTileKind::Slab {
            max_hp,
            armor_protection,
            armor_hardness,
        } => format!(
            "Slab\nHP: {}\nArmor: {}\nHardness: {}",
            **max_hp, **armor_protection, **armor_hardness
        ),
    };
    format!("{name}\nKey: {}\n{kind}", **key)
}

/// Render the wall/cover/scatter structural stat lines (HP / armor / hardness / band) shared by
/// the three structural [`CatalogTileKind`] variants.
fn structural_lines(stats: &StructuralStats) -> String {
    format!(
        "HP: {}\nArmor: {}\nHardness: {}\nBand: {:?}",
        *stats.max_hp, *stats.armor_protection, *stats.armor_hardness, stats.height_band,
    )
}

/// One palette row's [`Node`]: a full-width flex ROW (sprite left, name right), centred on the
/// cross axis, with a little padding + gap so the rows read as distinct entries. Relative units
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
/// units so it scales with the window (the portrait-node responsive precedent).
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

/// The [`StatText`] node's [`Node`]: full-width with a little padding so the stat lines don't
/// touch the panel edge. Relative units (no fixed `Px`).
fn stat_text_node() -> Node {
    Node {
        width: Val::Percent(100.0),
        padding: UiRect::all(Val::Vh(1.0)),
        ..default()
    }
}
