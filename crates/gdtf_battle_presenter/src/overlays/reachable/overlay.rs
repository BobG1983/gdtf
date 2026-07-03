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
//! the set each frame (the UI-mutate-not-respawn convention). The walk itself is the
//! shared [`draw_pool`](crate::overlays::pool::draw_pool) helper (GTW-568), which owns
//! the `set_if_neq` visibility flips.

use bevy::{camera::visibility::RenderLayers, prelude::*};
use gdtf_battle_sim::{Cell, CellLevel, Level, Tu};

use crate::{
    ActiveLevel, CELL_PX, Layer, WORLD_RENDER_LAYER, cell_to_world_layered,
    overlays::pool::draw_pool,
};

/// The environment variable that opts the reachable-range debug overlay IN
/// (GTW-450 C3): set truthy (`1` / `true` / `yes` / `on`, case-insensitive) to
/// render the overlay in a debug build. Unset / empty / any other value leaves it
/// off — the shipping default (the click-to-target route preview is the only move
/// feedback).
///
/// Read ONCE at startup into [`ReachableOverlayEnabled`] by
/// [`TopDownRendererPlugin`](crate::TopDownRendererPlugin); the overlay systems
/// `run_if` that resource's value, so tests set the RESOURCE directly and never
/// touch process-global env (the flaky-tests rule).
#[cfg(debug_assertions)]
pub const REACHABLE_OVERLAY_ENV: &str = "GDTF_DEBUG_REACHABLE_OVERLAY";

/// Whether the reachable-range DEBUG overlay renders this process (GTW-450 C3).
///
/// A named domain flag (no-bare-types: a `bool` carrying the "render the debug
/// overlay" meaning, private inner, read through the derived [`Deref`]). Seeded ONCE
/// at startup from [`REACHABLE_OVERLAY_ENV`] by
/// [`TopDownRendererPlugin`](crate::TopDownRendererPlugin); both the input-crate
/// populate system and the presenter draw system `run_if` `**flag` is `true`. Its
/// [`Default`] is `false` (overlay off) so a focused harness that omits the seed
/// gets the shipping behaviour.
///
/// Lives behind `#[cfg(debug_assertions)]` — the overlay it gates is debug-only, so
/// in a release build neither the flag nor the systems that read it compile (C1).
#[cfg(debug_assertions)]
#[derive(Resource, Debug, Clone, Copy, Default, PartialEq, Eq, Deref)]
pub struct ReachableOverlayEnabled(bool);

#[cfg(debug_assertions)]
impl ReachableOverlayEnabled {
    /// Build the flag from a raw enabled `bool` (the env-seed / test path).
    #[must_use]
    pub const fn new(enabled: bool) -> Self {
        Self(enabled)
    }

    /// Read [`REACHABLE_OVERLAY_ENV`] into the flag: truthy (`1` / `true` / `yes` /
    /// `on`, case-insensitive, trimmed) → enabled; unset / empty / anything else →
    /// disabled. Called ONCE at startup (NOT per-frame).
    #[must_use]
    pub fn from_env() -> Self {
        Self(std::env::var(REACHABLE_OVERLAY_ENV).is_ok_and(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        }))
    }
}

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

    // The world position of a reachable-cell sprite (shared by the reuse + grow paths).
    let world_at = |cell: CellLevel| {
        cell_to_world_layered(
            Cell::new(cell.x, cell.y),
            active_level,
            Layer::ReachableRange,
        )
    };
    // The shared pooled-draw walk (GTW-568): reuse the pooled sprites in iteration order
    // (move), lazily spawn past the pool, hide the surplus — the helper owns the
    // set_if_neq visibility flips (mutate, not respawn).
    draw_pool(
        sprites.iter_mut(),
        draws,
        |cell, (transform, _)| transform.translation = world_at(cell),
        |cell| spawn_reachable_sprite(&mut commands, world_at(cell)),
        |(_, visibility)| visibility,
    );
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
