//! The map-editor **right-panel controls** — the theme dropdown + the size selector that
//! populate the GTW-417 [`RightPanelRegion`](crate::RightPanelRegion) (GTW-421).
//!
//! Spawns, under the right scroll panel, the two authoring controls that drive the shared
//! [`MapEditorSession`]:
//!
//! - a THEME dropdown (the GTW-410 [`spawn_dropdown`] + [`register_dropdown::<LevelTheme>`])
//!   listing all three [`LevelTheme`] variants with [`LevelTheme::default`] pre-selected (C1),
//! - a SIZE selector of THREE numeric fields (the GTW-411 [`spawn_numeric_field`] +
//!   [`register_numeric_field`]) — width / height / levels — each clamped to its sim ceiling
//!   (`60` / `60` / `8`) and each committing into the session's [`GridSize`] (C3).
//!
//! The drive systems (`apply_theme_selection` / `apply_size_commit`) read the widgets'
//! commit messages and write the [`MapEditorSession`]. The contract names THREE clamped
//! fields because "the drawable area within 60×60×8" is three-dimensional (logged design
//! choice — a single field can't express a 3-axis extent).

use bevy::{prelude::*, ui::FlexDirection};
use gdtf_battle_sim::level::{
    GridHeight, GridLevels, GridSize, GridWidth, LevelTheme, MAX_GRID_SPAN, ThemeCatalogRegistry,
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
/// [`DropdownSelectionChanged<LevelTheme>`] back to THIS editor's theme control (no-bare-types
/// unit marker).
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub struct ThemeDropdown;

/// `OnEnter(Editing)`: spawn the theme dropdown + the three size fields under the right panel.
///
/// Runs after [`spawn_editor_shell`](crate::regions::spawn_editor_shell) (so the
/// [`RightPanelRegion`] exists) and gated on the live [`GdtfTheme`] (for the control colors).
/// Pre-selects [`LevelTheme::default`] in the dropdown (C1) and seeds each size field from the
/// full `60×60×8` [`GridSize::default`] (C3) — the SAME value `insert_session` seeds the
/// shared [`MapEditorSession`] to, so the controls and the session agree on open WITHOUT a
/// cross-system command-flush dependency (the session resource is inserted via deferred
/// `Commands` in the same schedule, so reading it here would race). The controls are parented
/// under the [`RightPanelRegion`]'s [`ScrollListArea`] — the frame's CLIPPING, SCROLLING
/// viewport child, NOT the grid root frame the marker rides — via a deferred command (the
/// scroll list is parented within the same buffer by `spawn_editor_shell`, so the re-parent
/// must wait — the `parent_scroll_root_under` precedent). Parenting onto the area (not the
/// frame) is what lets the controls stack from the TOP and scroll on overflow; parenting onto
/// the grid frame dropped them into an off-screen implicit grid row (the GTW-421 bottom-cramp).
pub(crate) fn spawn_right_panel_controls(mut commands: Commands, theme: Res<GdtfTheme>) {
    let dropdown_colors = DropdownColors {
        control_bg: *theme.panel.color,
        text:       *theme.text.text_color,
        popup_bg:   *theme.panel.color,
        option_bg:  *theme.panel.border_color,
    };
    let field_colors = FieldColors {
        background: *theme.panel.color,
        text:       *theme.text.text_color,
        caret:      *theme.text.text_color,
    };
    let label_color = *theme.text.text_color;

    // C1: every LevelTheme variant, the default pre-selected. The dropdown keeps its OWN
    // widget node (row layout + padding) — we do NOT pass a `Node` in the marker bundle,
    // which would clobber it and collapse the label (the GTW-421 clip bug). The control is
    // stretched to the field group's full width below so its `SpaceBetween` label has room.
    let options = vec![
        DropdownOption::new(LevelTheme::IndustrialHive, "Industrial Hive"),
        DropdownOption::new(LevelTheme::Underhive, "Underhive"),
        DropdownOption::new(LevelTheme::SumpWaste, "Sump Waste"),
    ];
    let selected = default_theme_index(&options);
    let dropdown = spawn_dropdown(
        &mut commands,
        options,
        selected,
        dropdown_colors,
        ThemeDropdown,
    );
    let theme_group = spawn_field_group(&mut commands, "Theme", label_color, dropdown);

    // C3: three clamped size fields seeded from the full-extent default (matching the
    // session `insert_session` seeds to). Each field is wrapped in its OWN labeled group so
    // width / height / levels read as three DISTINCT, identifiable rows (the GTW-421
    // overlap/occlusion bug — they previously stacked label-less and indistinct).
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
    // scroll panel (flex column + `JustifyContent::Start` — not pushed to the bottom, the
    // GTW-421 bottom-cramp bug), with full width + a little padding so the groups breathe.
    let content = commands
        .spawn(content_column_node())
        .add_children(&[theme_group, width_group, height_group, levels_group])
        .id();

    commands.queue(move |world: &mut World| {
        // The `RightPanelRegion` marker rides the scroll-list ROOT FRAME — a 2-column CSS
        // grid (content column + scrollbar column). Parenting the controls directly under
        // the frame drops them into an implicit grid ROW below the scroll area, OFF-screen
        // (measured: the content column sat at the panel's bottom edge, fully clipped — the
        // GTW-421 bottom-cramp). The controls must hang on the `ScrollListArea` — the frame's
        // CLIPPING, SCROLLING viewport child — so they stack from the TOP and the panel
        // scrolls them when they overflow. Find the area among the region frame's children.
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
///
/// The group OWNS the layout the widget marker no longer carries: it stretches the control
/// to full width (`AlignItems::Stretch`) so the dropdown's `SpaceBetween` label fits on one
/// line and the size fields read as distinct boxed rows, and it adds the per-field label so
/// width / height / levels (+ theme) are each identifiable. The label is a domain UI string,
/// not a bare value — it labels a control, so a plain `Text` child is the framework idiom.
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

/// The index of [`LevelTheme::default`] in the option list, or `0` if (impossibly) absent —
/// the dropdown pre-selects it on open (C1).
fn default_theme_index(options: &[DropdownOption<LevelTheme>]) -> usize {
    let default = LevelTheme::default();
    options
        .iter()
        .position(|opt| *opt.id() == default)
        .unwrap_or(0)
}

/// The top-aligned content COLUMN [`Node`] that stacks the labeled field groups down from
/// the TOP of the right scroll panel.
///
/// A flex COLUMN with `JustifyContent::Start` (top-aligned — the controls were previously
/// cramped at the bottom) and `AlignItems::Stretch` (each group fills the panel width).
/// Full panel width + a little inner padding so the groups don't touch the panel edge.
/// Relative units (`Percent` / `Vh`) per the responsive-UI rule.
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
/// column's width and stretches its control to that width, so the dropdown label fits on one
/// line and each size field reads as a distinct boxed row. Relative units (no fixed `Px`).
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
/// Reads [`DropdownSelectionChanged<LevelTheme>`] from THIS editor's [`ThemeDropdown`], sets
/// the session theme, and resolves the chosen theme's default-floor [`TileKey`] from the
/// GTW-409 [`ThemeCatalogRegistry`] — writing it into the session. A theme with no catalog
/// (or an absent registry) leaves the default floor unset rather than panicking
/// (bevy-traps #1: the registry is taken as `Option<Res<…>>`).
pub(crate) fn apply_theme_selection(
    mut changes: MessageReader<DropdownSelectionChanged<LevelTheme>>,
    dropdowns: Query<(), With<ThemeDropdown>>,
    registry: Option<Res<ThemeCatalogRegistry>>,
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
        let default_floor = registry
            .as_deref()
            .and_then(|registry| registry.catalog(theme))
            .map(|catalog| catalog.default_floor_key().clone());
        session.select_theme(theme, default_floor);
    }
}

/// `Update` (in `Editing`): fold a size-field commit into the session's [`GridSize`] (C3).
///
/// Reads [`NumericFieldCommitted<GridSpanInput>`], maps the committing field to its
/// [`SizeFieldAxis`], rebuilds the [`GridSize`] with that axis replaced, and re-validates via
/// [`GridSize::new`] — so the stored size is always in-bounds. The numeric field already
/// clamped the value to its `1..=ceiling` [`NumericRange`], so `GridSize::new` here is the
/// fail-closed backstop: on the (now-unreachable) error path the previous size is kept,
/// never a panic.
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
