//! The map-editor **right-panel controls** — the theme dropdown + the size selector that
//! populate the GTW-417 [`RightPanelRegion`](crate::RightPanelRegion) (GTW-421; swept onto the
//! UUID model in GTW-495).
//!
//! Spawns, under the right scroll panel, the two authoring controls that drive the shared
//! [`MapEditorSession`]:
//!
//! - a THEME dropdown (the GTW-410 [`spawn_dropdown`] + [`register_dropdown::<ThemeUuid>`])
//!   listing every theme the [`UuidThemeRegistry`](gdtf_battle_sim::level::UuidThemeRegistry)
//!   holds, BY its [`UuidThemeDef`](gdtf_battle_sim::level::UuidThemeDef) `display_name`, with
//!   the first theme (by display name) pre-selected (C1),
//! - a SIZE selector of THREE numeric fields (the GTW-411 [`spawn_numeric_field`] +
//!   [`register_numeric_field`]) — width / height / levels — each clamped to its sim ceiling
//!   (`60` / `60` / `8`) and each committing into the session's [`GridSize`] (C3).
//!
//! The drive systems (`apply_theme_selection` / `apply_size_commit`) read the widgets' commit
//! messages and write the [`MapEditorSession`]. [`seed_default_theme`] eagerly seeds the
//! session's theme to the dropdown's pre-selected default once the registries resolve (the
//! session is inserted via deferred `Commands` in the same `OnEnter` buffer, so the controls
//! cannot seed it directly — the GTW-421 command-flush rule).

use bevy::{prelude::*, ui::FlexDirection};
use gdtf_battle_sim::level::{
    GridHeight, GridLevels, GridSize, GridWidth, MAX_GRID_SPAN, ThemeUuid, UuidThemeRegistry,
};
use gdtf_ui::{
    DropdownColors, DropdownOption, DropdownSelectionChanged, FieldColors, NumericFieldCommitted,
    NumericRange, ScrollListArea, spawn_dropdown, spawn_numeric_field, theme::GdtfTheme,
};

use crate::{RightPanelRegion, session::MapEditorSession};

/// The `MAX_LEVELS` storey ceiling (the sim's z bound) — `8` (`docs/combat/battle-space.md`).
///
/// Named locally (not imported) only because `gdtf_battle_sim::metric::MAX_LEVELS` is the
/// canonical source; the size field clamps against the same magnitude the sim's
/// [`GridSize::new`] validates against.
const MAX_LEVELS: u8 = 8;

/// One drawable-area dimension's edited span, in cells — the
/// [`NumericValue`](gdtf_ui::NumericValue) the size fields edit, clamp, and commit (GTW-421).
///
/// A named newtype over [`u8`] (no-bare-types rule 1: a numeric-field generic is a domain
/// value, never a bare `u8`). Private inner + derived [`Deref`]; built through
/// [`new`](GridSpanInput::new). It implements exactly the
/// [`NumericValue`](gdtf_ui::NumericValue) bound set (`Copy + PartialOrd + FromStr + Display`)
/// so the three size fields are `spawn_numeric_field::<GridSpanInput>` — one shared input
/// type, each field carrying its OWN [`NumericRange`] ceiling and identity marker. Converted
/// into the axis newtypes ([`GridWidth`] / [`GridHeight`] / [`GridLevels`]) when a commit is
/// folded into the session's [`GridSize`].
#[derive(Deref, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct GridSpanInput(u8);

impl GridSpanInput {
    /// Wrap a cell span.
    #[must_use]
    pub const fn new(cells: u8) -> Self {
        Self(cells)
    }
}

impl core::fmt::Display for GridSpanInput {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl core::str::FromStr for GridSpanInput {
    type Err = core::num::ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse::<u8>().map(Self)
    }
}

/// Which drawable-area axis a size [`NumericField`](gdtf_ui::NumericField) drives — the
/// identity marker the caller attaches to each field so `apply_size_commit` maps a commit to
/// the right [`GridSize`] axis (GTW-421).
///
/// A named marker enum (no-bare-types) rather than three separate unit markers: the commit
/// reader matches on it in one place. Each field carries exactly one variant.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub enum SizeFieldAxis {
    /// The width field (x span, clamped `1..=`[`MAX_GRID_SPAN`]).
    Width,
    /// The height field (y span, clamped `1..=`[`MAX_GRID_SPAN`]).
    Height,
    /// The levels field (z span, clamped `1..=8`, the sim's `MAX_LEVELS` storey ceiling).
    Levels,
}

/// Marker on the theme dropdown's closed-control root, so `apply_theme_selection` can map a
/// [`DropdownSelectionChanged<ThemeUuid>`] back to THIS editor's theme control (no-bare-types
/// unit marker).
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub struct ThemeDropdown;

/// `OnEnter(Editing)`: spawn the theme dropdown + the three size fields under the right panel.
///
/// Runs after [`spawn_editor_shell`](crate::regions::spawn_editor_shell) (so the
/// [`RightPanelRegion`] exists) and gated on the live [`GdtfTheme`] (for the control colors).
/// Lists every theme the [`UuidThemeRegistry`] holds, sorted by display name for determinism,
/// pre-selecting the first (C1); a registry with no themes yields an empty dropdown rather than
/// panicking. Seeds each size field from the full `60×60×8` [`GridSize::default`] (C3). The
/// session theme is seeded separately by [`seed_default_theme`] (the session is inserted via a
/// deferred command in the same `OnEnter` buffer, so reading it here would race — the GTW-421
/// command-flush rule). The controls are parented under the [`RightPanelRegion`]'s
/// [`ScrollListArea`] — the frame's CLIPPING, SCROLLING viewport child — via a deferred command
/// so they top-anchor and scroll on overflow (the GTW-421 bottom-cramp fix).
pub(crate) fn spawn_right_panel_controls(
    mut commands: Commands,
    theme: Res<GdtfTheme>,
    themes: Option<Res<UuidThemeRegistry>>,
) {
    let dropdown_colors = DropdownColors {
        control_bg:          *theme.panel.color,
        text:                *theme.text.text_color,
        popup_bg:            *theme.panel.color,
        option_bg:           *theme.panel.border_color,
        // The hovered / focused / selected option's highlight — the theme's hover fill, distinct
        // from the resting `option_bg` so the active row reads as highlighted (GTW-499).
        option_highlight_bg: *theme.button.hover,
    };
    let field_colors = FieldColors {
        background: *theme.panel.color,
        text:       *theme.text.text_color,
        caret:      *theme.text.text_color,
    };
    let label_color = *theme.text.text_color;

    // C1: every theme the UuidThemeRegistry holds, listed by its display_name, sorted by label
    // for a deterministic order, the first pre-selected. The dropdown keeps its OWN widget node
    // (row layout + padding) — we do NOT pass a `Node` in the marker bundle, which would clobber
    // it and collapse the label (the GTW-421 clip bug).
    let options = theme_options(themes.as_deref());
    let dropdown = spawn_dropdown(&mut commands, options, 0, dropdown_colors, ThemeDropdown);
    let theme_group = spawn_field_group(&mut commands, "Theme", label_color, dropdown);

    // C3: three clamped size fields seeded from the full-extent default (matching the session
    // `insert_session` seeds to). Each field is wrapped in its OWN labeled group so width /
    // height / levels read as three DISTINCT, identifiable rows.
    let size = GridSize::default();
    let width = spawn_size_field(
        &mut commands,
        *size.width(),
        MAX_GRID_SPAN,
        field_colors,
        SizeFieldAxis::Width,
    );
    let width_group = spawn_field_group(&mut commands, "Width", label_color, width);
    let height = spawn_size_field(
        &mut commands,
        *size.height(),
        MAX_GRID_SPAN,
        field_colors,
        SizeFieldAxis::Height,
    );
    let height_group = spawn_field_group(&mut commands, "Height", label_color, height);
    let levels = spawn_size_field(
        &mut commands,
        *size.levels(),
        MAX_LEVELS,
        field_colors,
        SizeFieldAxis::Levels,
    );
    let levels_group = spawn_field_group(&mut commands, "Levels", label_color, levels);

    // A top-aligned content COLUMN that stacks the labeled groups down from the TOP of the
    // scroll panel (flex column + `JustifyContent::Start`), with full width + a little padding
    // so the groups breathe.
    let content = commands
        .spawn(content_column_node())
        .add_children(&[theme_group, width_group, height_group, levels_group])
        .id();

    commands.queue(move |world: &mut World| {
        // The `RightPanelRegion` marker rides the scroll-list ROOT FRAME — a 2-column CSS grid
        // (content column + scrollbar column). The controls must hang on the `ScrollListArea` —
        // the frame's CLIPPING, SCROLLING viewport child — so they stack from the TOP and the
        // panel scrolls them when they overflow (the GTW-421 bottom-cramp).
        let Some(frame) = world
            .query_filtered::<Entity, With<RightPanelRegion>>()
            .iter(world)
            .next()
        else {
            return;
        };
        let Some(children) = world.get::<Children>(frame) else {
            return;
        };
        let area = children.iter().find(|child| {
            world
                .get_entity(*child)
                .is_ok_and(|entity| entity.contains::<ScrollListArea>())
        });
        let Some(area) = area else {
            return;
        };
        if let Ok(mut area_entity) = world.get_entity_mut(area) {
            area_entity.add_child(content);
        }
    });
}

/// Build the theme dropdown's option list from the [`UuidThemeRegistry`] — one
/// [`DropdownOption<ThemeUuid>`] per registered theme, labeled by its
/// [`UuidThemeDef`](gdtf_battle_sim::level::UuidThemeDef) `display_name`, sorted by label so the
/// order is deterministic (the registry is a `HashMap`). An absent / empty registry yields an
/// empty list (the dropdown then offers nothing rather than panicking).
fn theme_options(themes: Option<&UuidThemeRegistry>) -> Vec<DropdownOption<ThemeUuid>> {
    let Some(themes) = themes else {
        return Vec::new();
    };
    let mut options: Vec<(String, ThemeUuid)> = themes
        .defs()
        .map(|(key, def)| ((*def.display_name).clone(), *key))
        .collect();
    options.sort_by(|a, b| a.0.cmp(&b.0));
    options
        .into_iter()
        .map(|(label, key)| DropdownOption::new(key, label))
        .collect()
}

/// `Update` (in `Editing`): seed the session theme to the dropdown's pre-selected default once
/// the [`UuidThemeRegistry`] resolves (C1).
///
/// The session opens with the [`ThemeUuid::nil`] sentinel theme; this picks the first theme (by
/// display name — the SAME order the dropdown pre-selects index `0` of) and resolves its
/// default-floor key, writing both into the session. Runs while the session theme is still nil
/// (it seeds exactly once), guarded on the optional registry / session (bevy-traps #1). After
/// the seed the palette / canvas populate from a real theme rather than the empty nil theme.
pub(crate) fn seed_default_theme(
    themes: Option<Res<UuidThemeRegistry>>,
    session: Option<ResMut<MapEditorSession>>,
) {
    let (Some(themes), Some(mut session)) = (themes, session) else {
        return;
    };
    if !session.theme().is_nil() {
        return;
    }
    let mut entries: Vec<(String, ThemeUuid)> = themes
        .defs()
        .map(|(key, def)| ((*def.display_name).clone(), *key))
        .collect();
    entries.sort_by(|a, b| a.0.cmp(&b.0));
    let Some((_, theme)) = entries.into_iter().next() else {
        return;
    };
    let default_floor = themes.default_floor(&theme);
    session.select_theme(theme, default_floor);
}

/// Spawn one clamped size field: `initial` cells, clamped `1..=ceiling`, tagged with its
/// [`SizeFieldAxis`]. The `1` lower bound is the sim's minimum (a grid is at least `1×1×1` —
/// [`GridSize::new`] rejects zero). The field keeps its OWN widget node (row + padding +
/// min-width); we pass ONLY the axis marker so the marker bundle never clobbers it.
fn spawn_size_field(
    commands: &mut Commands,
    initial: u8,
    ceiling: u8,
    colors: FieldColors,
    axis: SizeFieldAxis,
) -> Entity {
    spawn_numeric_field(
        commands,
        GridSpanInput::new(initial),
        NumericRange::new(GridSpanInput::new(1), GridSpanInput::new(ceiling)),
        colors,
        axis,
    )
}

/// Wrap one control widget (`control`) in a labeled field GROUP: a flex COLUMN holding a
/// small text `label` above the control, stretched to the panel width, with vertical
/// breathing room between groups (GTW-421 layout fix).
fn spawn_field_group(
    commands: &mut Commands,
    label: &str,
    label_color: Color,
    control: Entity,
) -> Entity {
    let label_node = commands
        .spawn((Text::new(label), TextColor(label_color), label_node()))
        .id();
    commands
        .spawn(field_group_node())
        .add_children(&[label_node, control])
        .id()
}

/// The top-aligned content COLUMN [`Node`] that stacks the labeled field groups down from
/// the TOP of the right scroll panel.
fn content_column_node() -> Node {
    Node {
        width: Val::Percent(100.0),
        flex_direction: FlexDirection::Column,
        justify_content: JustifyContent::Start,
        align_items: AlignItems::Stretch,
        padding: UiRect::all(Val::Vh(1.0)),
        row_gap: Val::Vh(1.5),
        ..default()
    }
}

/// One field GROUP's [`Node`]: a flex COLUMN (label above control) that fills the content
/// column's width and stretches its control to that width.
fn field_group_node() -> Node {
    Node {
        width: Val::Percent(100.0),
        flex_direction: FlexDirection::Column,
        align_items: AlignItems::Stretch,
        row_gap: Val::Vh(0.4),
        ..default()
    }
}

/// A field-group LABEL's [`Node`]: a little bottom margin so the label sits just above its
/// control without crowding. Relative units (no fixed `Px`).
fn label_node() -> Node {
    Node {
        margin: UiRect::bottom(Val::Vh(0.2)),
        ..default()
    }
}

/// `Update` (in `Editing`): fold a theme selection into the [`MapEditorSession`] (C2).
///
/// Reads [`DropdownSelectionChanged<ThemeUuid>`] from THIS editor's [`ThemeDropdown`], sets the
/// session theme, and resolves the chosen theme's default-floor [`TerrainUuid`] from the
/// GTW-487 [`UuidThemeRegistry`] — writing it into the session. A theme with no registered
/// default floor (or an absent registry) leaves the default floor unset rather than panicking
/// (bevy-traps #1: the registry is taken as `Option<Res<…>>`).
pub(crate) fn apply_theme_selection(
    mut changes: MessageReader<DropdownSelectionChanged<ThemeUuid>>,
    dropdowns: Query<(), With<ThemeDropdown>>,
    themes: Option<Res<UuidThemeRegistry>>,
    session: Option<ResMut<MapEditorSession>>,
) {
    let Some(mut session) = session else {
        return;
    };
    for change in changes.read() {
        if dropdowns.get(change.control()).is_err() {
            continue;
        }
        let theme = *change.id();
        let default_floor = themes
            .as_deref()
            .and_then(|themes| themes.default_floor(&theme));
        session.select_theme(theme, default_floor);
    }
}

/// `Update` (in `Editing`): fold a size-field commit into the session's [`GridSize`] (C3).
///
/// Reads [`NumericFieldCommitted<GridSpanInput>`], maps the committing field to its
/// [`SizeFieldAxis`], rebuilds the [`GridSize`] with that axis replaced, and re-validates via
/// [`GridSize::new`] — so the stored size is always in-bounds. The numeric field already
/// clamped the value to its `1..=ceiling` [`NumericRange`], so `GridSize::new` here is the
/// fail-closed backstop: on the (now-unreachable) error path the previous size is kept, never a
/// panic.
pub(crate) fn apply_size_commit(
    mut commits: MessageReader<NumericFieldCommitted<GridSpanInput>>,
    field_axes: Query<&SizeFieldAxis>,
    session: Option<ResMut<MapEditorSession>>,
) {
    let Some(mut session) = session else {
        return;
    };
    for commit in commits.read() {
        let Ok(target) = field_axes.get(commit.field()) else {
            continue;
        };
        let cells = *commit.value().value();
        let current = session.grid_size();
        let (width, height, levels) = match target {
            SizeFieldAxis::Width => (GridWidth::new(cells), current.height(), current.levels()),
            SizeFieldAxis::Height => (current.width(), GridHeight::new(cells), current.levels()),
            SizeFieldAxis::Levels => (current.width(), current.height(), GridLevels::new(cells)),
        };
        if let Ok(next) = GridSize::new(width, height, levels) {
            session.set_grid_size(next);
        }
    }
}
