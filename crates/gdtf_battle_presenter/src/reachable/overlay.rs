//! The reachable-range overlay (E7 · GTW-12i): the presenter-owned read-seam
//! [`ReachableOverlay`] resource, its two draw systems
//! ([`draw_reachable_overlay`] — the per-cell tint sprites; [`draw_reachable_labels`]
//! — the per-cell TU-cost [`Text2d`] labels), and their cell-keyed sprite/label markers.
//!
//! # The C4 read-seam (input → presenter → sim)
//!
//! The PRESENTER owns the read-seam ([`ReachableOverlay`], a
//! `Vec<(`[`CellLevel`]`, `[`Tu`]`)>`) plus these draw systems; the INPUT crate (which
//! alone may read `SelectedShooter` + the selected ganger's `Position` / `Tu`, and which
//! builds the GTW-353 [`PlanningView`](gdtf_battle_sim::PlanningView) from the sim's
//! `SquadVisibility`) calls [`reachable_within`](gdtf_battle_sim::reachable_within) and
//! POPULATES this resource (clearing it when nothing is selected). Selection is NEVER
//! pushed into the authoritative sim model, and the dependency direction stays
//! `input → presenter → sim` — the SAME shape as the
//! [`HighlightRequest`](crate::HighlightRequest) seam (the presenter DEFINES the type; the
//! input crate WRITES it). The reachable set the input crate computes uses the SAME
//! `reachable_within` + `PlanningView` the move dispatch
//! ([`dispatch_move`](gdtf_battle_sim::acts::dispatch_move)) uses, so the lit set EXACTLY
//! matches the cells a commit will accept (GTW-354 dependency).
//!
//! # Hard-cut to the active storey (the GTW-347 fog precedent)
//!
//! Both draw systems hard-cut to the presenter's [`ActiveLevel`](crate::ActiveLevel): an
//! overlay cell on the active storey is shown; an off-storey cell is
//! [`Visibility::Hidden`]. The overlay sits on the [`Layer::ReachableOverlay`] band (above
//! the highlight) so it composites over the terrain / actors / reticle.
//!
//! # Mutate, never respawn (the UI-mutate-not-respawn convention)
//!
//! Each system maintains a POOL of cell-keyed sprites / labels: it reuses an existing
//! entity for the cell it now needs (moving its [`Transform`], rewriting its text / colour,
//! showing it) and HIDES surplus pooled entities it no longer needs — it never
//! despawn-then-respawns the set each frame (the mirror of [`present_fog`](crate::present_fog)
//! mutating the terrain material in place).

use bevy::{
    camera::visibility::RenderLayers, ecs::system::SystemParam, prelude::*, text::TextColor,
};
use gdtf_battle_sim::{Cell, CellLevel, Level, Tu};

use crate::{ActiveLevel, CELL_PX, Layer, WORLD_RENDER_LAYER, cell_to_world_layered};

/// The presenter-owned reachable-range read-seam — the cells the SELECTED ganger can
/// reach this commit, each paired with the cheapest move TU cost to reach it (C1 / C4).
///
/// A named newtype over the `Vec<(`[`CellLevel`]`, `[`Tu`]`)>` the sim's
/// [`reachable_within`](gdtf_battle_sim::reachable_within) returns (no-bare-types: the
/// reachable set is a domain value), [`Deref`]ing to its inner slice so the draw systems
/// iterate it directly. OWNED BY THE PRESENTER so the `input → presenter → sim` direction
/// holds: the draw systems READ it; the input crate's
/// [`populate_reachable_overlay`](../../../gdtf_battle_input) POPULATES it (the
/// [`HighlightRequest`](crate::HighlightRequest) precedent). `init_resource`-d by the
/// [`TopDownRendererPlugin`](crate::TopDownRendererPlugin), so its [`Default`] is the
/// empty set (nothing selected → nothing lit).
#[derive(Resource, Deref, Debug, Clone, Default, PartialEq, Eq)]
pub struct ReachableOverlay(Vec<(CellLevel, Tu)>);

impl ReachableOverlay {
    /// Build a reachable overlay holding `cells` — each a `(cell, cheapest-TU)` pair from
    /// [`reachable_within`](gdtf_battle_sim::reachable_within).
    #[must_use]
    pub const fn new(cells: Vec<(CellLevel, Tu)>) -> Self {
        Self(cells)
    }

    /// The empty (nothing-reachable) overlay — what the input populate system writes when
    /// no ganger is selected.
    #[must_use]
    pub const fn cleared() -> Self {
        Self(Vec::new())
    }
}

/// Marker for a pooled reachable-overlay TINT [`Sprite`].
///
/// Plumbing around the framework sprite (the no-bare-types framework carve-out, the same
/// justification the [`HoverHighlight`](crate::HoverHighlight) /
/// [`SelectionHighlight`](gdtf_battle_input) markers use):
/// [`draw_reachable_overlay`] queries `With<ReachableTint>` to find and MUTATE the pooled
/// tint sprites in place rather than despawn-respawning them each frame.
#[derive(Component, Debug, Default, Clone, Copy, Eq, PartialEq, Hash)]
pub struct ReachableTint;

/// Marker for a pooled reachable-overlay TU-cost [`Text2d`] label.
///
/// Plumbing around the framework world-space text (the no-bare-types framework carve-out):
/// [`draw_reachable_labels`] queries `With<ReachableLabel>` to find and MUTATE the pooled
/// labels in place (rewrite the text, move the [`Transform`], show/hide) rather than
/// despawn-respawning them each frame.
#[derive(Component, Debug, Default, Clone, Copy, Eq, PartialEq, Hash)]
pub struct ReachableLabel;

/// The translucent tint of a reachable-overlay cell.
///
/// Framework plumbing — a literal [`Color`] handed straight to a [`Sprite`], not a domain
/// quantity (the `CELL_PX`-class const carve-out, the [`HoverHighlight`](crate::HoverHighlight)
/// tint precedent). A cool green at moderate alpha so the reachable cells read as a "you can
/// move here" wash over the terrain without hiding the tile beneath, distinct from the warm
/// hover reticle + the cyan selection reticle.
const REACHABLE_TINT: Color = Color::srgba(0.35, 0.9, 0.45, 0.3);

/// The colour of the TU-cost label text — an opaque near-white so the cost reads against
/// the green tint beneath it.
const LABEL_COLOR: Color = Color::srgb(0.95, 1.0, 0.95);

/// The label font size, in world-space px.
///
/// Framework plumbing — a layout scalar fed straight to a [`TextFont`], not a domain
/// quantity (the `CELL_PX`-class carve-out, the FCT [`spawn_floating_text`](crate::spawn_floating_text)
/// precedent). Small relative to [`CELL_PX`] (`16.0`) so a two-digit cost fits over one cell.
const LABEL_FONT_PX: f32 = 9.0;

/// The world-space vertical lift of a TU-cost label ABOVE its cell centre, in world px.
///
/// The user OQ-5 ruling: the per-target TU cost reads ABOVE the target cell. Lifts the
/// label by just over half a cell so it sits over the top edge of the tinted cell.
const LABEL_LIFT_PX: f32 = CELL_PX * 0.55;

/// `Update` ([`PresenterSystems::Draw`](crate::PresenterSystems)): draw the reachable-range
/// TINT overlay — one cell-keyed [`Sprite`] per reachable cell on the active storey (C2).
///
/// Reads the presenter-owned [`ReachableOverlay`] read-seam (populated by the input crate,
/// C4) and the [`ActiveLevel`], then maintains a POOL of [`ReachableTint`] sprites:
///
/// - for each reachable cell ON the active storey, it takes (or lazily spawns) a pooled
///   sprite, moves it to [`cell_to_world_layered`] at the [`Layer::ReachableOverlay`] band,
///   tints it [`REACHABLE_TINT`], and shows it;
/// - every surplus pooled sprite (off-storey cells, or pooled entities beyond the current
///   set) is [`Visibility::Hidden`] — NEVER despawned (the UI-mutate-not-respawn convention,
///   the [`present_fog`](crate::present_fog) precedent).
///
/// Off-[`ActiveLevel`] cells are NOT drawn — the hard cut to the active storey (the GTW-347
/// fog precedent / AC4). Sized to one cell (`custom_size: Some(Vec2::splat(CELL_PX))`) and
/// drawn on [`RenderLayers::layer`]`(`[`WORLD_RENDER_LAYER`]`)` so it composites with the
/// battlefield, not the GTW-120 UI camera.
///
/// Param-only (`bevy-traps.md` #7): [`Commands`] for the lazy pool growth, the
/// [`ReachableOverlay`] / [`ActiveLevel`] reads, and a
/// `Query<(&mut Transform, &mut Visibility), With<ReachableTint>>` for the in-place
/// move + show/hide. Battle-gated + in [`PresenterSystems::Draw`](crate::PresenterSystems)
/// by the [`TopDownRendererPlugin`](crate::TopDownRendererPlugin).
pub fn draw_reachable_overlay(
    mut commands: Commands,
    overlay: Res<ReachableOverlay>,
    active: Res<ActiveLevel>,
    mut tints: Query<(&mut Transform, &mut Visibility), With<ReachableTint>>,
) {
    // The cells to light THIS update — only those on the active storey (the hard cut).
    let active_level: Level = **active;
    let lit: Vec<CellLevel> = overlay
        .iter()
        .filter(|(cell, _)| cell.z == i32::from(*active_level))
        .map(|(cell, _)| *cell)
        .collect();

    // Reuse the pooled tint sprites in iteration order: move + show the first `lit.len()`,
    // hide the rest. A deterministic pairing (cell-by-index) is fine — every lit cell gets
    // exactly one sprite and the surplus is hidden, so no duplicate / stale tint shows.
    let mut pooled = tints.iter_mut();
    for cell in &lit {
        let world = cell_to_world_layered(
            Cell::new(cell.x, cell.y),
            active_level,
            Layer::ReachableOverlay,
        );
        if let Some((mut transform, mut visibility)) = pooled.next() {
            transform.translation = world;
            *visibility = Visibility::Visible;
        } else {
            spawn_tint(&mut commands, world);
        }
    }
    // Hide every surplus pooled sprite the current set no longer needs.
    for (_, mut visibility) in pooled {
        *visibility = Visibility::Hidden;
    }
}

/// Lazily spawn ONE pooled reachable-tint sprite at `world`.
///
/// A one-cell translucent [`REACHABLE_TINT`] [`Sprite`] on the world render layer, shown
/// from spawn. Pooled (kept + reused / hidden, never despawned), so this runs only when the
/// reachable set grows past the current pool size.
fn spawn_tint(commands: &mut Commands, world: Vec3) {
    commands.spawn((
        ReachableTint,
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

/// `Update` ([`PresenterSystems::Draw`](crate::PresenterSystems)): draw the per-cell TU-cost
/// LABELS — one world-space [`Text2d`] above each reachable cell on the active storey (C3).
///
/// Reads the same presenter-owned [`ReachableOverlay`] read-seam + [`ActiveLevel`] as
/// [`draw_reachable_overlay`], then maintains a POOL of [`ReachableLabel`] [`Text2d`] labels:
///
/// - for each reachable `(cell, cost)` ON the active storey, it takes (or lazily spawns) a
///   pooled label, REWRITES its text to the cost (the exact [`Tu`] from
///   [`reachable_within`](gdtf_battle_sim::reachable_within)), moves it to
///   [`cell_to_world_layered`] at the [`Layer::ReachableOverlay`] band lifted by
///   [`LABEL_LIFT_PX`] (the user OQ-5 "ABOVE the cell" ruling), and shows it;
/// - every surplus pooled label is [`Visibility::Hidden`] — NEVER despawned (mutate, not
///   respawn).
///
/// Off-[`ActiveLevel`] cells are NOT drawn (the hard cut, AC4). The label is the FCT
/// [`Text2d`] precedent ([`spawn_floating_text`](crate::spawn_floating_text)): world-space,
/// anchored over the cell, on [`WORLD_RENDER_LAYER`]. DENSITY FLAG (C3): a label per
/// reachable cell is intentionally dense per the user's AC; once a hovered-cell cursor exists
/// (GTW-356 / GTW-358) this may move to a cursor-only label — but THIS slice draws the literal
/// per-cell set the AC specifies.
///
/// Param-only (`bevy-traps.md` #7): [`Commands`] for the lazy pool growth, the
/// [`ReachableOverlay`] / [`ActiveLevel`] reads, and a [`LabelPool`] bundling the
/// pooled-label query (the `&mut Text2d` + `&mut Transform` + `&mut Visibility` it mutates in
/// place).
pub fn draw_reachable_labels(
    mut commands: Commands,
    overlay: Res<ReachableOverlay>,
    active: Res<ActiveLevel>,
    mut pool: LabelPool,
) {
    let active_level: Level = **active;
    // The `(cell, cost)` pairs to label THIS update — only on the active storey (hard cut).
    let lit: Vec<(CellLevel, Tu)> = overlay
        .iter()
        .filter(|(cell, _)| cell.z == i32::from(*active_level))
        .copied()
        .collect();

    let mut pooled = pool.labels.iter_mut();
    for (cell, cost) in &lit {
        let mut world = cell_to_world_layered(
            Cell::new(cell.x, cell.y),
            active_level,
            Layer::ReachableOverlay,
        );
        // The user OQ-5 ruling: the cost reads ABOVE the cell.
        world.y += LABEL_LIFT_PX;
        let text = label_text(*cost);
        if let Some((mut label, mut transform, mut visibility)) = pooled.next() {
            // Rewrite the pooled label's text in place (mutate, not respawn).
            **label = text;
            transform.translation = world;
            *visibility = Visibility::Visible;
        } else {
            spawn_label(&mut commands, text, world);
        }
    }
    for (_, _, mut visibility) in pooled {
        *visibility = Visibility::Hidden;
    }
}

/// The pooled-label query bundle for [`draw_reachable_labels`].
///
/// A [`SystemParam`] grouping the `&mut Text2d` + `&mut Transform` + `&mut Visibility`
/// query over the pooled [`ReachableLabel`] entities — keeps the system signature small (the
/// three-component label mutation is one logical access). Framework plumbing (a query bundle),
/// exempt from no-bare-types.
#[derive(SystemParam)]
pub struct LabelPool<'w, 's> {
    /// The pooled labels: each carries its [`Text2d`] (rewritten in place), [`Transform`]
    /// (moved to the cell), and [`Visibility`] (shown / hidden).
    labels: Query<
        'w,
        's,
        (
            &'static mut Text2d,
            &'static mut Transform,
            &'static mut Visibility,
        ),
        With<ReachableLabel>,
    >,
}

/// The label text for a move cost — the bare TU count followed by `" TU"` (the firemode /
/// status-panel TU convention).
///
/// `pub(super)` so the sibling `reachable::test` module can pin the label format without an
/// app harness.
pub(super) fn label_text(cost: Tu) -> String {
    format!("{} TU", *cost)
}

/// Lazily spawn ONE pooled reachable-cost label [`Text2d`] showing `text` at `world`.
///
/// A world-space [`Text2d`] anchored bottom-centre over the cell (so it sits ABOVE the cell,
/// the user OQ-5 ruling), in [`LABEL_COLOR`] at [`LABEL_FONT_PX`], on the world render layer.
/// Pooled (kept + reused / hidden, never despawned), so this runs only when the reachable set
/// grows past the current label pool.
fn spawn_label(commands: &mut Commands, text: String, world: Vec3) {
    commands.spawn((
        ReachableLabel,
        Text2d::new(text),
        TextFont {
            font_size: bevy::text::FontSize::Px(LABEL_FONT_PX),
            ..default()
        },
        TextColor(LABEL_COLOR),
        bevy::sprite::Anchor::BOTTOM_CENTER,
        Transform::from_translation(world),
        Visibility::Visible,
        RenderLayers::layer(WORLD_RENDER_LAYER),
    ));
}
