//! The reachable-range overlay (GTW-387 C3): the presenter-owned read-seam
//! [`ReachableCells`] resource (the cells the selected ganger can reach within its
//! remaining TU), its draw system ([`draw_reachable_overlay`]), and the cell-keyed
//! sprite marker.
//!
//! # The C6 read-seam (input → presenter → sim)
//!
//! The PRESENTER owns the read-seam ([`ReachableCells`], the reachable `(cell, level)` +
//! accumulated TU-cost list) plus this draw system; the INPUT crate calls
//! [`reachable_within`](gdtf_battle_sim::reachable_within) for the SELECTED ganger and
//! POPULATES this resource (clearing it when no ganger is selected). Selection and
//! [`ActiveLevel`](crate::ActiveLevel) are NEVER pushed into the authoritative sim model
//! — the dependency direction stays `input → presenter → sim`, the SAME shape as the
//! [`PathPreview`](crate::PathPreview) seam.
//!
//! # Active-storey hard-cut
//!
//! [`draw_reachable_overlay`] reads [`ActiveLevel`](crate::ActiveLevel) live every frame
//! and shows ONLY the reachable cells whose storey equals the active level. After a
//! `PageUp` to L1 the draw system renders the L1 members of the reachable set — no extra
//! wiring needed, the resource holds the FULL cross-storey reachable set. The input crate
//! recomputes it from the ganger's actual position (may be L0) and `reachable_within`
//! returns the spanning L0 + L1 set (via the now-fixed link traversal, GTW-387 B).
//!
//! # Mutate, never respawn
//!
//! The system maintains a POOL of [`ReachableCellSprite`]-marked sprites: it reuses an
//! existing entity for each reachable cell to draw (moving its [`Transform`], showing it)
//! and HIDES surplus pooled entities it no longer needs — it never despawn-then-respawns
//! the set each frame (the UI-mutate-not-respawn convention, the
//! [`present_fog`](crate::present_fog) / [`draw_path_preview`](crate::draw_path_preview)
//! precedent).

use bevy::{camera::visibility::RenderLayers, prelude::*};
use gdtf_battle_sim::{Cell, CellLevel, Level, Tu};

use crate::{ActiveLevel, CELL_PX, Layer, WORLD_RENDER_LAYER, cell_to_world_layered};

/// The presenter-owned reachable-range overlay read-seam — the cells the SELECTED
/// ganger can reach within its remaining TU, each with its cheapest accumulated cost
/// (GTW-387 C3, the `input → presenter → sim` seam).
///
/// A named domain value (no-bare-types: the reachable set is a domain value; the
/// `(CellLevel, Tu)` pair is the domain cost-annotated cell). `init_resource`-d by the
/// [`TopDownRendererPlugin`](crate::TopDownRendererPlugin), so its [`Default`] is the
/// empty set (no selection → nothing drawn). The draw system hard-cuts to the active
/// storey; the input crate holds the FULL cross-storey reachable set here.
#[derive(Resource, Debug, Clone, Default, PartialEq, Eq)]
pub struct ReachableCells(Vec<ReachableCell>);

impl ReachableCells {
    /// Build the reachable set from the `(CellLevel, Tu)` pairs returned by
    /// [`reachable_within`](gdtf_battle_sim::reachable_within).
    #[must_use]
    pub fn new(cells: impl IntoIterator<Item = (CellLevel, Tu)>) -> Self {
        Self(
            cells
                .into_iter()
                .map(|(cell, cost)| ReachableCell { cell, cost })
                .collect(),
        )
    }

    /// The empty (no-selection / no-battle) reachable set — what the input populate
    /// system writes when no ganger is selected.
    #[must_use]
    pub const fn cleared() -> Self {
        Self(Vec::new())
    }

    /// The reachable cells, as `(CellLevel, Tu)` pairs.
    pub fn cells(&self) -> impl Iterator<Item = (CellLevel, Tu)> + '_ {
        self.0.iter().map(|r| (r.cell, r.cost))
    }

    /// Whether the set is empty (no ganger selected or no cells reachable).
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// One entry in the reachable-range set — a `(cell, level)` the selected ganger can
/// reach, paired with its cheapest accumulated TU cost.
///
/// A named pair (no-bare-types: the `(CellLevel, Tu)` pair is a domain value — the
/// reachable cell annotated with its entry cost).
#[derive(Debug, Clone, PartialEq, Eq)]
struct ReachableCell {
    /// The `(cell, level)` reachable within the ganger's TU budget.
    cell: CellLevel,
    /// The cheapest accumulated TU cost to reach `cell`.
    cost: Tu,
}

/// Marker for a pooled reachable-range [`Sprite`].
///
/// Plumbing around the framework sprite (the no-bare-types framework carve-out, the same
/// justification the [`PathStepSprite`](crate::PathStepSprite) marker uses):
/// [`draw_reachable_overlay`] queries `With<ReachableCellSprite>` to find and MUTATE the
/// pooled sprites in place rather than despawn-respawning them each frame.
#[derive(Component, Debug, Default, Clone, Copy, Eq, PartialEq, Hash)]
pub struct ReachableCellSprite;

/// The translucent tint of a reachable-range cell.
///
/// Framework plumbing — a literal [`Color`] handed straight to a [`Sprite`], not a domain
/// quantity (the `CELL_PX`-class const carve-out). A cool green at 40% alpha so the
/// reachable floor reads as a "this is where you could move" highlight distinct from the
/// amber route-preview trail.
const REACHABLE_TINT: Color = Color::srgba(0.2, 0.9, 0.4, 0.4);

/// The pooled reachable-range sprite query — mutated in place over the
/// [`ReachableCellSprite`] pool. A `type` alias so the system signature stays under the
/// `type_complexity` lint. Framework plumbing (a query alias), exempt from no-bare-types.
type ReachableSpriteQuery<'w, 's> =
    Query<'w, 's, (&'static mut Transform, &'static mut Visibility), With<ReachableCellSprite>>;

/// Resolve the reachable-range sprites to draw for the current active storey — the PURE
/// draw-decision helper that clips the full cross-storey [`ReachableCells`] to the
/// active level.
///
/// Returns every [`CellLevel`] in `reachable` whose storey index equals `active_level`.
/// Called by [`draw_reachable_overlay`] and directly tested by the unit tests in the
/// sibling `test` module without an [`App`].
///
/// `pub(super)` so the sibling `test` module can pin the hard-cut resolution.
pub(super) fn reachable_draws(reachable: &ReachableCells, active_level: Level) -> Vec<CellLevel> {
    let active_z = i32::from(*active_level);
    reachable
        .cells()
        .map(|(cell, _cost)| cell)
        .filter(|cell| cell.z == active_z)
        .collect()
}

/// `Update` ([`PresenterSystems::Draw`](crate::PresenterSystems)): draw the
/// reachable-range overlay — one cell-keyed [`Sprite`] per reachable cell on the active
/// storey (GTW-387 C3).
///
/// Reads the presenter-owned [`ReachableCells`] read-seam (populated by the input crate,
/// C6) and the [`ActiveLevel`] (the active-storey hard-cut), then maintains a POOL of
/// [`ReachableCellSprite`] sprites:
///
/// - for each reachable cell ON the active storey, it takes (or lazily spawns) a pooled
///   sprite, moves it to [`cell_to_world_layered`] at the [`Layer::ReachableRange`] band,
///   and shows it;
/// - every surplus pooled sprite is [`Visibility::Hidden`] — NEVER despawned (the
///   UI-mutate-not-respawn convention).
///
/// The overlay follows `PageUp` with NO extra wiring: it reads `Res<ActiveLevel>` live
/// every frame, so after `PageUp` raises `ActiveLevel` to L1, the next Draw renders only
/// L1 reachable cells.
///
/// Param-only (`bevy-traps.md` #7): [`Commands`] for the lazy pool growth, the
/// [`ReachableCells`] / [`ActiveLevel`] reads, and the [`ReachableSpriteQuery`] for the
/// pooled sprites. Battle-gated in [`TopDownRendererPlugin`](crate::TopDownRendererPlugin)
/// by [`PresenterSystems::Draw`](crate::PresenterSystems).
pub fn draw_reachable_overlay(
    mut commands: Commands,
    reachable: Res<ReachableCells>,
    active: Res<ActiveLevel>,
    mut sprites: ReachableSpriteQuery,
) {
    let active_level: Level = **active;
    let draws = reachable_draws(&reachable, active_level);

    // Reuse the pooled sprites in iteration order: move + show the first `draws.len()`,
    // hide the rest (mutate, not respawn — the present_fog / draw_path_preview precedent).
    let mut pooled = sprites.iter_mut();
    for cell in &draws {
        let world = cell_to_world_layered(
            Cell::new(cell.x, cell.y),
            active_level,
            Layer::ReachableRange,
        );
        if let Some((mut transform, mut visibility)) = pooled.next() {
            transform.translation = world;
            *visibility = Visibility::Visible;
        } else {
            spawn_reachable_sprite(&mut commands, world);
        }
    }
    // Hide every surplus pooled sprite the current set no longer needs.
    for (_, mut visibility) in pooled {
        *visibility = Visibility::Hidden;
    }
}

/// Lazily spawn ONE pooled reachable-range step sprite at `world`.
///
/// A one-cell translucent [`Sprite`] on the world render layer, shown from spawn. Pooled
/// (kept + reused / hidden, never despawned), so this runs only when the reachable set
/// grows past the current pool size.
fn spawn_reachable_sprite(commands: &mut Commands, world: Vec3) {
    commands.spawn((
        ReachableCellSprite,
        Sprite {
            color: REACHABLE_TINT,
            custom_size: Some(Vec2::splat(CELL_PX)),
            ..default()
        },
        Transform::from_translation(world),
        Visibility::Visible,
        RenderLayers::layer(WORLD_RENDER_LAYER),
    ));
}
