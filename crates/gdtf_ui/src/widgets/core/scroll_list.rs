//! The [`ScrollList`] widget: a reusable, clipping, vertically-scrollable list
//! container (GTW-412).
//!
//! A scroll list is the LAST Wave-0 building block: a themed frame that CLIPS and
//! SCROLLS a vertical stack of caller-supplied rows taller than the frame, with a
//! [`Scrollbar`](bevy::ui_widgets::Scrollbar) the user drags / wheels. The list is
//! OPAQUE to what a row IS — the caller spawns its own row bundles and parents them
//! onto the scroll-area entity this builder returns; the list only STACKS (a flex
//! column) + CLIPS (`overflow: scroll_y`) them, so the gang-member list, the tile
//! palette, and the stat panels each fill it with their own content.
//!
//! ## Built on the engine scroll widgets — never hand-rolled
//!
//! The scroll mechanism is Bevy's built-in
//! [`ScrollArea`](bevy::ui_widgets::ScrollArea) +
//! [`Scrollbar`](bevy::ui_widgets::Scrollbar) (the `bevy_ui_widgets` crate, in scope
//! via the workspace `ui` feature — NO new dependency). The
//! [`ScrollAreaPlugin`](bevy::ui_widgets::ScrollAreaPlugin) observer reads the
//! wheel ([`Pointer<Scroll>`](bevy::picking::events::Pointer)) and clamps
//! [`ScrollPosition`](bevy::ui::ScrollPosition) to the overflow; the
//! [`ScrollbarPlugin`](bevy::ui_widgets::ScrollbarPlugin)'s `update_scrollbar_thumb`
//! (in `PostUpdate`, after `ui_layout_system`) sizes + positions the thumb from the
//! content/viewport ratio. [`UiPlugin`](crate::UiPlugin) ensures both plugins are
//! present (they ride `DefaultPlugins`' `UiWidgetsPlugins`; `UiPlugin` adds them only
//! if absent, so a bare app still gets them — see
//! [`UiPlugin::build`](crate::UiPlugin)).
//!
//! ## Relative sizing, with ONE documented API-forced chrome exception
//!
//! Every layout dimension is RELATIVE (`Percent`/`Vw`/`Vh`/flex) per the responsive-UI
//! rule — the frame, the content column, and the scroll-area cell all flex to their
//! parent. The SOLE fixed-pixel values are the scrollbar chrome
//! ([`SCROLLBAR_WIDTH_PX`] and [`SCROLLBAR_MIN_THUMB_PX`]): Bevy's scroll API DEFINES
//! these as pixels — [`Node::scrollbar_width`](bevy::ui::Node::scrollbar_width) and
//! [`Scrollbar::min_thumb_length`](bevy::ui_widgets::Scrollbar::min_thumb_length) are
//! both `f32` pixel quantities by API contract, with no relative-unit form. They are
//! kept minimal and are the unavoidable widget-API chrome carve-out to the
//! no-fixed-px rule (the same class as a font size), documented on each const.
//!
//! ## Out of scope (explicit)
//!
//! The expand/collapse lerp ACCORDION is NOT this widget (it is GTW-403 child /
//! GTW-416) and is deliberately not built here. Focus-scroll (scrolling a
//! keyboard-focused row into view via
//! [`ScrollIntoView`](bevy::ui_widgets::ScrollIntoView)) is left as a deferred hook,
//! not wired: the consumers (gang list / tile palette) are pointer-driven, so it is
//! out of the GTW-412 AC set. Scrollbar auto-hide when the content fits is also NOT
//! built (the engine has no built-in for it and AC1 does not require it) — the bar is
//! always present, sized by the engine to fill the track when nothing overflows.

use bevy::{
    prelude::*,
    ui::{
        BackgroundColor, BorderRadius, Display, FlexDirection, GridPlacement, Node, Overflow,
        RepeatedGridTrack, ScrollPosition, UiRect, Val,
    },
    ui_widgets::{ControlOrientation, ScrollArea, Scrollbar, ScrollbarThumb},
};

/// The color set a [`ScrollList`] paints itself with.
///
/// Pure UI plumbing ([`Color`](bevy::prelude::Color)s): the scroll-area cell's fill,
/// the scrollbar track's fill, and the draggable thumb's fill. Mirrors the sanctioned
/// `*Colors` builder pattern of [`SwitchColors`](super::SwitchColors) /
/// [`DropdownColors`](super::DropdownColors). The derived [`Default`] (all-transparent)
/// is a spawn-seed sentinel only — a builder always supplies real colors.
#[derive(Component, Clone, Copy, PartialEq, Debug, Default)]
pub struct ScrollListColors {
    /// The scroll-area (clipping content viewport) background fill.
    pub area:  Color,
    /// The scrollbar track background fill.
    pub track: Color,
    /// The draggable scrollbar thumb fill.
    pub thumb: Color,
}

/// Marker on the ROOT frame of a [`ScrollList`] (the 2-column grid: content + scrollbar).
///
/// A unit marker — presence alone is the signal (no-bare-types rule). The caller may
/// attach its own identity marker alongside it; the builder also accepts a `marker`
/// bundle so a caller can find its own list.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct ScrollList;

/// Marker on the SCROLL-AREA cell of a [`ScrollList`] — the clipping, scrolling content
/// column the caller parents its rows onto.
///
/// This is the [`Entity`] [`spawn_scroll_list`] returns. It carries
/// [`ScrollArea`](bevy::ui_widgets::ScrollArea), `overflow: scroll_y`, and an explicit
/// [`ScrollPosition`](bevy::ui::ScrollPosition); the engine observer scrolls it. A unit
/// marker (no-bare-types rule).
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct ScrollListArea;

/// Marker on the SCROLLBAR track cell of a [`ScrollList`].
///
/// Carries the [`Scrollbar`](bevy::ui_widgets::Scrollbar) targeting the
/// [`ScrollListArea`] and one [`ScrollbarThumb`](bevy::ui_widgets::ScrollbarThumb)
/// child. A unit marker (no-bare-types rule).
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct ScrollListBar;

/// Spawns a themed [`ScrollList`] and returns the SCROLL-AREA [`Entity`] the caller
/// parents its rows onto (`commands.entity(area).add_child(row)` / `set_parent`).
///
/// `colors` are the area / track / thumb fills; `marker` is any [`Bundle`] the caller
/// wants on the ROOT frame — typically its own identity marker so it can find this
/// list later. The returned entity is the SCROLL AREA (not the root), because the
/// list's whole job is to stack + clip the rows the caller hangs there.
///
/// The frame is a 2-column CSS grid — a `flex(1.0)` content column (the scroll area)
/// and an `auto` scrollbar column — so the bar sits BESIDE the content, never over it.
/// The scroll-area cell is a flex COLUMN with `overflow: scroll_y` (so a taller stack
/// clips + scrolls), [`ScrollArea`](bevy::ui_widgets::ScrollArea), an explicit
/// [`ScrollPosition`](bevy::ui::ScrollPosition)`(Vec2::ZERO)` (so the engine has a
/// position to clamp from frame 0), and its `scrollbar_width` set to
/// [`SCROLLBAR_WIDTH_PX`] — equal to the track column's `min_width` so the bar reserves
/// exactly its own width and does not overlay content (the engine gotcha). The
/// scrollbar cell carries [`Scrollbar`](bevy::ui_widgets::Scrollbar) (vertical,
/// targeting the area, with [`SCROLLBAR_MIN_THUMB_PX`]) and one
/// [`ScrollbarThumb`](bevy::ui_widgets::ScrollbarThumb) child that carries NO
/// [`Node`](bevy::ui::Node) — the engine writes the thumb's
/// [`ComputedNode`](bevy::ui::ComputedNode) / transform directly, so giving it a `Node`
/// would fight the scrollbar system.
pub fn spawn_scroll_list(
    commands: &mut Commands,
    colors: ScrollListColors,
    marker: impl Bundle,
) -> Entity {
    // The clipping, scrolling content column. Spawned first so the scrollbar can target
    // its entity id. `ScrollArea` `#[require]`s `ScrollPosition`, but we seed it
    // explicitly so the engine has a value to clamp against on the very first frame.
    let area = commands
        .spawn((
            ScrollListArea,
            ScrollArea,
            ScrollPosition(Vec2::ZERO),
            BackgroundColor(colors.area),
            scroll_area_node(),
        ))
        .id();
    // The scrollbar track cell + its single thumb child. The thumb carries NO `Node`
    // (the engine owns its layout); only its rounded border styling rides the marker.
    let thumb = commands
        .spawn((
            ScrollbarThumb {
                border_radius: BorderRadius::all(Val::Percent(THUMB_RADIUS_PCT)),
                border:        UiRect::ZERO,
            },
            BackgroundColor(colors.thumb),
        ))
        .id();
    let bar = commands
        .spawn((
            ScrollListBar,
            Scrollbar {
                target:           area,
                orientation:      ControlOrientation::Vertical,
                min_thumb_length: SCROLLBAR_MIN_THUMB_PX,
            },
            BackgroundColor(colors.track),
            scrollbar_track_node(),
        ))
        .id();
    commands.entity(bar).add_child(thumb);
    // The 2-column grid frame holding the content column + the scrollbar column.
    let root = commands
        .spawn((ScrollList, colors, scroll_list_frame_node()))
        .id();
    commands.entity(root).add_children(&[area, bar]);
    commands.entity(root).insert(marker);
    area
}

/// The root frame [`Node`](bevy::ui::Node): a 2-column CSS grid — a `flex(1.0)` content
/// column and an `auto` scrollbar column — that FILLS its parent (`100%` both axes), so
/// the caller sizes the list purely by sizing its parent cell. All-relative.
fn scroll_list_frame_node() -> Node {
    Node {
        display: Display::Grid,
        width: Val::Percent(100.0),
        height: Val::Percent(100.0),
        // One flex content column (takes all spare width) + one auto scrollbar column
        // (sizes to the track's `min_width`).
        grid_template_columns: vec![RepeatedGridTrack::flex(1, 1.0), RepeatedGridTrack::auto(1)],
        ..default()
    }
}

/// The scroll-area cell [`Node`](bevy::ui::Node): a flex COLUMN that clips + scrolls on
/// the Y axis. Placed in grid column 1. `scrollbar_width` (the API-forced chrome px)
/// reserves the bar's gutter so it does not overlay content. All other dimensions are
/// relative (it fills its grid cell).
fn scroll_area_node() -> Node {
    Node {
        grid_column: GridPlacement::start(1),
        display: Display::Flex,
        flex_direction: FlexDirection::Column,
        width: Val::Percent(100.0),
        height: Val::Percent(100.0),
        overflow: Overflow::scroll_y(),
        // API-forced chrome px: reserve exactly the bar's own width so the bar (in the
        // sibling grid column) never overlays the scrolling content (the engine gotcha).
        scrollbar_width: SCROLLBAR_WIDTH_PX,
        ..default()
    }
}

/// The scrollbar track cell [`Node`](bevy::ui::Node): a full-height column placed in
/// grid column 2, exactly [`SCROLLBAR_WIDTH_PX`] wide (the API-forced chrome px, equal
/// to the scroll area's reserved `scrollbar_width`). Height is relative (`100%`).
fn scrollbar_track_node() -> Node {
    Node {
        grid_column: GridPlacement::start(2),
        height: Val::Percent(100.0),
        // API-forced chrome px: matches the scroll area's `scrollbar_width` so the bar
        // fills exactly the reserved gutter.
        min_width: Val::Px(SCROLLBAR_WIDTH_PX),
        ..default()
    }
}

/// The scrollbar track + thumb thickness, in LOGICAL PIXELS.
///
/// This is the ONE px class beyond a font size the relative-sizing rule (C2) admits: a
/// FORCED widget-API chrome value. Bevy's scroll API defines the bar gutter as a pixel
/// quantity — [`Node::scrollbar_width`](bevy::ui::Node::scrollbar_width) is an `f32`
/// pixel field with NO relative-unit form, and the track cell's
/// [`min_width`](bevy::ui::Node::min_width) must equal it (set as
/// [`Val::Px`](bevy::ui::Val::Px)) so the bar reserves — and exactly fills — its own
/// gutter rather than overlaying the content. Kept minimal at a thin 8px bar. It is NOT
/// a domain value (framework plumbing fed straight to the engine), so it is a bare `f32`
/// like the existing z-band consts.
const SCROLLBAR_WIDTH_PX: f32 = 8.0;

/// The scrollbar thumb's MINIMUM length along the scroll axis, in LOGICAL PIXELS.
///
/// The second (and last) API-forced chrome px: [`Scrollbar::min_thumb_length`](bevy::ui_widgets::Scrollbar::min_thumb_length)
/// is an `f32` pixel quantity by API contract — the floor under the engine-computed thumb
/// size (track length × visible/content) so a list with a huge content:viewport ratio
/// still shows a grabbable thumb instead of a vanishing sliver. No relative-unit form
/// exists; kept minimal at 24px. Framework plumbing, not a domain value (a bare `f32`).
const SCROLLBAR_MIN_THUMB_PX: f32 = 24.0;

/// The scrollbar thumb's corner rounding, as a PERCENT of its own size — a relative unit
/// (NOT the px chrome exception). A soft pill so the thumb reads as a handle.
const THUMB_RADIUS_PCT: f32 = 40.0;
