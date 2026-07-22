//! The cross-level-signals DRAW system (GTW-596): compact corner badges rendered
//! ON the active storey, change-driven off EITHER [`CrossLevelSignals`] OR
//! [`ActiveLevel`] (a badge's drawn Z-band is hard-cut to the active storey, so a
//! level switch must redraw even when the two storeys' derived signal sets
//! happen to be identical — see [`draw_cross_level_signals`]).
//!
//! # ART DEPENDENCY (placeholder art, per the ticket's explicit allowance)
//!
//! No shipped chevron / pip / drop-marker glyph exists yet (~6 new 16px glyphs
//! are the eventual real art). This draw uses PLACEHOLDER solid-color tiles
//! ([`THREAT_TINT`] / [`DROP_DEPTH_TINT`] / [`CONNECTOR_TINT`]) plus a plain-ASCII
//! [`Text2d`] label ([`badge_label`]) instead of a Unicode chevron glyph — ASCII
//! `+`/`-`/`v` need no special glyph coverage from the bundled font (the risk a
//! Unicode arrow character carries), and the contract's OWN worked example
//! (`"+2 x3"`) is itself exactly this ASCII sign+magnitude+count shape. QA should
//! treat every tint/label here as swappable placeholder, not final art.

use bevy::{camera::visibility::RenderLayers, prelude::*, sprite::Anchor, text::FontSize};
use gdtf_battle_sim::prelude::{Cell, Level};

use super::types::{CrossLevelBadgeKind, CrossLevelSignals};
use crate::{
    ActiveLevel, Layer, WORLD_RENDER_LAYER, cell_to_world_layered, overlays::pool::draw_pool,
};

/// Marker for a pooled cross-level-badge background [`Sprite`] (the placeholder
/// solid-color tile — see the module doc's ART DEPENDENCY note).
#[derive(Component, Debug, Default, Clone, Copy, Eq, PartialEq, Hash)]
pub struct CrossLevelBadgeTile;

/// Marker for a pooled cross-level-badge [`Text2d`] label (the ASCII sign +
/// magnitude / count text — see `badge_label`).
#[derive(Component, Debug, Default, Clone, Copy, Eq, PartialEq, Hash)]
pub struct CrossLevelBadgeLabel;

/// The pooled badge-TILE query for [`draw_cross_level_signals`] — `Without` the
/// label marker so the two pooled-entity queries are provably disjoint (no B0001
/// conflict, `bevy-traps.md` #3 family). A `type` alias so the system signature
/// stays under the `type_complexity` lint. Framework plumbing (a query alias),
/// exempt from no-bare-types.
type TileQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static mut Sprite,
        &'static mut Transform,
        &'static mut Visibility,
    ),
    (With<CrossLevelBadgeTile>, Without<CrossLevelBadgeLabel>),
>;

/// The pooled badge-LABEL query for [`draw_cross_level_signals`] — `Without` the
/// tile marker, disjoint from [`TileQuery`]. Framework plumbing (a query alias),
/// exempt from no-bare-types.
type LabelQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static mut Text2d,
        &'static mut Transform,
        &'static mut Visibility,
    ),
    (With<CrossLevelBadgeLabel>, Without<CrossLevelBadgeTile>),
>;

/// The compact badge tile size, in world px — "compact 5-6px" per the contract.
const BADGE_SIZE_PX: f32 = 6.0;

/// The badge label's font size, in world px — small enough that a `"+2 x3"` label
/// fits within the badge's neighbourhood without overrunning the cell.
const BADGE_LABEL_FONT_PX: f32 = 5.0;

/// The tiny z-lift that keeps a badge's label drawing ABOVE its own background
/// tile (both otherwise share [`Layer::CrossLevelSignal`]'s z) — far smaller than
/// the gap to the next distinct [`Layer`] band, so it never crosses into another
/// band.
const LABEL_Z_LIFT: f32 = 0.001;

/// The up-to-[`BADGE_CAP_PER_CELL`](super::types::BADGE_CAP_PER_CELL) corner slot
/// offsets a cell's badges stack into — ONE corner (top-right), stacked downward,
/// per the contract's "compact ... corner badges".
const BADGE_SLOT_OFFSETS: [Vec2; 3] = [
    Vec2::new(5.0, 5.0),
    Vec2::new(5.0, -1.0),
    Vec2::new(5.0, -7.0),
];

/// Placeholder solid-color tint for a [`CrossLevelBadgeKind::Threat`] badge — a
/// warm red so an enemy signal reads as danger.
const THREAT_TINT: Color = Color::srgb(0.85, 0.2, 0.2);
/// Placeholder solid-color tint for a [`CrossLevelBadgeKind::DropDepth`] badge —
/// an amber/brown "hazard floor" tint.
const DROP_DEPTH_TINT: Color = Color::srgb(0.6, 0.42, 0.15);
/// Placeholder solid-color tint for a [`CrossLevelBadgeKind::ConnectorDelta`]
/// badge — a cool blue "connector" tint, distinct from the warm Threat /
/// `DropDepth` pair.
const CONNECTOR_TINT: Color = Color::srgb(0.25, 0.55, 0.9);
/// The badge label's text colour — opaque near-white so it reads against every
/// tint above.
const BADGE_LABEL_COLOR: Color = Color::srgb(0.95, 0.97, 0.95);

/// One resolved badge draw — its world position (the TILE's position; the label
/// lifts a hair above it, see [`label_world`]), background tint, and label text.
///
/// A named, `Clone`-able draw-list item (no-bare-types: a badge's on-screen
/// content is a domain value, not a bare tuple) so [`draw_cross_level_signals`]
/// can hand the SAME list to both pooled queries (tile + label).
#[derive(Debug, Clone, PartialEq)]
struct BadgeDraw {
    /// The badge tile's world position (`cell_to_world_layered` at
    /// [`Layer::CrossLevelSignal`], plus its corner-slot offset).
    world: Vec3,
    /// The placeholder solid-color tint (see [`badge_tint`]).
    tint:  Color,
    /// The ASCII sign+magnitude/count label text (see [`badge_label`]).
    label: String,
}

/// The badge tint for `kind` — the placeholder per-kind solid colour (see the
/// module-level ART DEPENDENCY note).
const fn badge_tint(kind: CrossLevelBadgeKind) -> Color {
    match kind {
        CrossLevelBadgeKind::Threat { .. } => THREAT_TINT,
        CrossLevelBadgeKind::DropDepth { .. } => DROP_DEPTH_TINT,
        CrossLevelBadgeKind::ConnectorDelta { .. } => CONNECTOR_TINT,
    }
}

/// The badge label text for `kind` — an ASCII sign + magnitude (the contract's
/// OWN `"+2 x3"` worked example), so no Unicode glyph coverage is required of the
/// bundled font:
///
/// - [`Threat`](CrossLevelBadgeKind::Threat): `<sign><magnitude>` plus
///   `" x<count>"` when more than one enemy shares the delta (`"+2 x3"`, `"-1"`).
/// - [`DropDepth`](CrossLevelBadgeKind::DropDepth): `v<storeys>` (`"v3"`).
/// - [`ConnectorDelta`](CrossLevelBadgeKind::ConnectorDelta): `<sign><magnitude>`
///   (`"+2"` ascend, `"-1"` descend) — distinguished from Threat by [`badge_tint`]
///   alone (a placeholder scheme; real art gets a distinct icon).
fn badge_label(kind: CrossLevelBadgeKind) -> String {
    match kind {
        CrossLevelBadgeKind::Threat { delta, count } => {
            let sign = if delta.is_above() { '+' } else { '-' };
            if *count > 1 {
                format!("{sign}{} x{}", delta.magnitude(), *count)
            } else {
                format!("{sign}{}", delta.magnitude())
            }
        }
        CrossLevelBadgeKind::DropDepth { storeys } => format!("v{}", *storeys),
        CrossLevelBadgeKind::ConnectorDelta { delta } => {
            let sign = if delta.is_above() { '+' } else { '-' };
            format!("{sign}{}", delta.magnitude())
        }
    }
}

/// Flatten [`CrossLevelSignals`] into this frame's ordered badge draw list — every
/// cell's (already capped) badges placed into [`BADGE_SLOT_OFFSETS`] in order.
///
/// Cells are visited in `(x, y)` order (not the resource's `HashMap` iteration
/// order) so the draw list — and therefore which pooled entity ends up drawing
/// which badge — is deterministic frame to frame.
fn badge_draws(signals: &CrossLevelSignals, active_level: Level) -> Vec<BadgeDraw> {
    let mut cells: Vec<Cell> = signals.cells().collect();
    cells.sort_by_key(|cell| (cell.x, cell.y));

    let mut draws = Vec::new();
    for cell in cells {
        for (kind, offset) in signals.badges_at(cell).iter().zip(BADGE_SLOT_OFFSETS) {
            let world = cell_to_world_layered(cell, active_level, Layer::CrossLevelSignal)
                + offset.extend(0.0);
            draws.push(BadgeDraw {
                world,
                tint: badge_tint(*kind),
                label: badge_label(*kind),
            });
        }
    }
    draws
}

/// The label's world position — its tile's position lifted [`LABEL_Z_LIFT`] so it
/// draws above the tile (same `(x, y)`, a hair more `z`).
fn label_world(tile_world: Vec3) -> Vec3 {
    Vec3::new(tile_world.x, tile_world.y, tile_world.z + LABEL_Z_LIFT)
}

/// `Update` ([`PresenterSystems::Overlay`](crate::PresenterSystems)): draw the
/// cross-level tactical badges (GTW-596) — up to
/// [`BADGE_CAP_PER_CELL`](super::types::BADGE_CAP_PER_CELL) compact tile+label
/// pairs per cell, stacked into ONE corner, drawn ON the active storey.
///
/// A badge's drawn WORLD POSITION is shaped by TWO independent inputs: the
/// [`CrossLevelSignals`] content (which cells/kinds to draw) AND [`ActiveLevel`]
/// (`cell_to_world_layered`'s storey Z-band — badges are hard-cut to the active
/// storey exactly like every other overlay). Gating the redraw on
/// `CrossLevelSignals` alone is NOT sufficient: two different active storeys can
/// legitimately derive an IDENTICAL signal set (e.g. two independent stair links
/// stacked at the same `(x, y)` each contributing the same `ConnectorDelta` from
/// their own active-storey endpoint), in which case `CrossLevelSignals` never
/// ticks `Changed` on the level switch — yet the badge must still redraw at the
/// NEW storey's Z-band, or it is left drawn at the OLD storey's band (invisible /
/// mis-layered once the view has actually moved). So the walk re-runs whenever
/// EITHER input changed — the [`draw_static_battlefield`](crate::draw_static_battlefield)
/// multi-trigger precedent (internal `is_changed()` early-return, not an
/// all-in-`run_if` gate, since a normal `Res` param already carries the change
/// tick this needs). Per UI convention this walk still MUTATES the pooled badges
/// in place; it never despawn-respawns. Each badge is TWO pooled entities sharing
/// one draw-list index — a [`CrossLevelBadgeTile`] (the placeholder solid-color
/// background) and a [`CrossLevelBadgeLabel`] (the label text, lifted a hair
/// above its own tile so it reads over it) — the
/// [`draw_fire_target`](crate::draw_fire_target) tile+label precedent,
/// generalised from a 0/1 draw list to an N-length one via the SAME
/// [`draw_pool`] walk called twice over the identical draw list.
///
/// Param-only (`bevy-traps.md` #7): [`Commands`] for the lazy pool growth, the
/// [`CrossLevelSignals`] / [`ActiveLevel`] reads, and the two disjoint pooled
/// queries (`Without` each other's marker — no B0001 conflict).
pub fn draw_cross_level_signals(
    mut commands: Commands,
    signals: Res<CrossLevelSignals>,
    active: Res<ActiveLevel>,
    mut tiles: TileQuery,
    mut labels: LabelQuery,
) {
    if !signals.is_changed() && !active.is_changed() {
        return;
    }

    let active_level: Level = **active;
    let draws = badge_draws(&signals, active_level);

    draw_pool(
        tiles.iter_mut(),
        draws.clone(),
        |draw: BadgeDraw, (sprite, transform, _)| {
            sprite.color = draw.tint;
            transform.translation = draw.world;
        },
        |draw: BadgeDraw| spawn_tile(&mut commands, draw.tint, draw.world),
        |(_, _, visibility)| visibility,
    );

    draw_pool(
        labels.iter_mut(),
        draws,
        |draw: BadgeDraw, (label, transform, _)| {
            transform.translation = label_world(draw.world);
            **label = Text2d::new(draw.label);
        },
        |draw: BadgeDraw| {
            let world = label_world(draw.world);
            spawn_label(&mut commands, draw.label, world);
        },
        |(_, _, visibility)| visibility,
    );
}

/// Lazily spawn ONE pooled cross-level-badge background tile [`Sprite`] at
/// `world`, tinted `tint`.
fn spawn_tile(commands: &mut Commands, tint: Color, world: Vec3) {
    commands.spawn((
        CrossLevelBadgeTile,
        Sprite {
            color: tint,
            custom_size: Some(Vec2::splat(BADGE_SIZE_PX)),
            ..default()
        },
        Transform::from_translation(world),
        Visibility::Visible,
        RenderLayers::layer(WORLD_RENDER_LAYER),
    ));
}

/// Lazily spawn ONE pooled cross-level-badge [`Text2d`] label showing `text` at
/// `world`.
fn spawn_label(commands: &mut Commands, text: String, world: Vec3) {
    commands.spawn((
        CrossLevelBadgeLabel,
        Text2d::new(text),
        TextFont {
            font_size: FontSize::Px(BADGE_LABEL_FONT_PX),
            ..default()
        },
        TextColor(BADGE_LABEL_COLOR),
        Anchor::CENTER,
        Transform::from_translation(world),
        Visibility::Visible,
        RenderLayers::layer(WORLD_RENDER_LAYER),
    ));
}
