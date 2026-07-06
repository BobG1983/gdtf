//! Input-seam overlay draw registration: hover highlight, route path preview, fire
//! target, area-damage field wash, and the debug-only reachable-range overlay.

use bevy::prelude::*;
use gdtf_battle_sim::{BattleInProgress, FieldRegistry, SquadVisibility};

use crate::{
    HighlightRequest, PresenterSystems, draw_field_overlay, draw_fire_target,
    draw_highlight_on_request, draw_path_preview,
};
// GTW-450 — the reachable-range overlay items are DEBUG-only (C1); imported via a
// separate `#[cfg(debug_assertions)]` `use` below so the release build never names them.
#[cfg(debug_assertions)]
use crate::{ReachableCells, ReachableOverlayEnabled, draw_reachable_overlay};

/// Registers the GTW-251 message-driven hover-highlight draw into the
/// [`PresenterSystems::Overlay`] stage (GTW-623 — the over-the-composed-scene stage).
///
/// The presenter DEFINES the [`HighlightRequest`] message (the consumer owns its input
/// API, mirroring how the sim defines the `*Requested` messages input writes) and
/// registers its buffer here via [`App::add_message`] (Bevy 0.18 — buffered events are
/// messages, `bevy-traps.md` #4). The buffer registration is UNCONDITIONAL (not behind
/// the asset gate): a [`MessageReader<HighlightRequest>`](bevy::ecs::message::MessageReader)
/// panics param validation without its `Messages<HighlightRequest>` buffer (`bevy-traps.md`
/// #4), and `add_message` is IDEMPOTENT — the input crate also registers the same buffer
/// so its `MessageWriter` validates headlessly, and the two coexist (the `*Requested`
/// precedent).
///
/// [`draw_highlight_on_request`] joins the shared `PresenterSystems::Overlay` stage
/// (configured once, chained after `Compose`), gated `run_if(resource_exists::<BattleInProgress>)`
/// — the sim's live-battle witness, the same gate the other draw systems use, so the
/// highlight is inert pre-battle (`bevy-traps.md` #1). It needs NO render resource (it
/// draws a solid-tint sprite, not an atlas tile) and its `Messages<HighlightRequest>`
/// buffer is guaranteed present by the `add_message` above, so the battle gate alone is
/// sufficient. The MIGRATED highlight sprite (the `HoverHighlight` marker + its lazy spawn)
/// now lives in `highlight.rs`; its lifecycle matches the old input-side one (lazily
/// spawned, despawned with the battle world).
pub(super) fn register_highlight_systems(app: &mut App) {
    app.add_message::<HighlightRequest>().add_systems(
        Update,
        draw_highlight_on_request
            .in_set(PresenterSystems::Overlay)
            .run_if(resource_exists::<BattleInProgress>),
    );
}

/// Registers the GTW-358 / GTW-368 route path-preview DRAW system into the
/// [`PresenterSystems::Overlay`] stage.
///
/// The PRESENTER owns the [`PathPreview`](crate::PathPreview) read-seam (`init_resource`-d
/// on build) plus this draw system; the INPUT crate POPULATES the resource by calling
/// [`find_path`](gdtf_battle_sim::find_path) for the selected ganger → the target — the
/// [`HighlightRequest`] precedent, where the presenter DEFINES the type and input WRITES it,
/// keeping the `input → presenter → sim` direction (never a cycle).
///
/// [`draw_path_preview`] draws each route step on the active storey with a pooled,
/// mutated-in-place [`Sprite`] (never despawn-respawned), §53-dimmed on a squad-EXPLORED
/// (remembered) step, plus a MINIMAL marker at the cell where the route leaves the active
/// storey (the C5 off-storey-continuation placeholder; the full cross-storey indicator is the
/// GTW-359 soft dep) AND the GTW-368 SINGLE TU-cost [`Text2d`] label at the previewed TARGET
/// cell (the route's last cell), mutated in place + hidden when the preview is empty.
///
/// Gated `run_if(resource_exists::<BattleInProgress>)` — the sim's live-battle witness, the
/// same gate the highlight draw uses (the inert-pre-battle requirement, `bevy-traps.md` #1) —
/// AND `resource_exists::<SquadVisibility>` (the §53 VISIBLE-vs-EXPLORED read: the fog sets are
/// inserted by the sim's `setup_battle`, absent in a focused harness that opens
/// `BattleInProgress` directly; without this guard the `Res<SquadVisibility>` param would panic
/// validation — the [`present_fog`](crate::present_fog) precedent). It needs NO render resource
/// (a solid-tint sprite and a `Text2d`, not an atlas tile) and the always-present
/// `init_resource`-d [`PathPreview`](crate::PathPreview) and [`ActiveLevel`](crate::ActiveLevel).
/// Its `Overlay` stage membership (chained after `Compose` — GTW-623) makes the route composite
/// OVER the fogged battlefield — the route preview is the topmost within-storey band
/// ([`Layer::PathPreview`](crate::Layer)).
pub(super) fn register_path_preview_systems(app: &mut App) {
    app.add_systems(
        Update,
        draw_path_preview.in_set(PresenterSystems::Overlay).run_if(
            resource_exists::<BattleInProgress>.and_then(resource_exists::<SquadVisibility>),
        ),
    );
}

/// Registers the GTW-371 fire-target highlight DRAW system into the
/// [`PresenterSystems::Overlay`] stage.
///
/// The PRESENTER owns the [`FireTargetHighlight`](crate::FireTargetHighlight) read-seam
/// (`init_resource`-d on build) plus this draw system; the INPUT crate POPULATES the resource
/// by deciding the fireable-enemy verdict (mirroring `decide_left_click`'s FIRE rung) +
/// computing the [`mode_tu_cost`](gdtf_battle_sim::mode_tu_cost) — the [`HighlightRequest`]
/// precedent, where the presenter DEFINES the type and input WRITES it, keeping the
/// `input → presenter → sim` direction (never a cycle).
///
/// [`draw_fire_target`] draws the SINGLE red tile UNDER the hovered enemy
/// ([`Layer::FireTarget`](crate::Layer), z below the actor band) with a pooled,
/// mutated-in-place [`Sprite`] (never despawn-respawned) plus the SINGLE OPAQUE TU-cost
/// [`Text2d`] label above the cell, hard-cut to the active storey and hidden off a fireable
/// hover.
///
/// Gated `run_if(resource_exists::<BattleInProgress>)` — the sim's live-battle witness, the same
/// gate the highlight / path-preview draws use (the inert-pre-battle requirement,
/// `bevy-traps.md` #1). It needs NO render resource (a solid-tint sprite + a `Text2d`, not an
/// atlas tile) and NO `SquadVisibility` (the input populate applies the fog gate before writing
/// the seam — the draw only reads the resolved highlight + the always-present `init_resource`-d
/// [`FireTargetHighlight`](crate::FireTargetHighlight) / [`ActiveLevel`](crate::ActiveLevel)). Its
/// `Overlay` stage membership (chained after `Compose` — GTW-623) makes the red tile composite
/// OVER the fogged battlefield (and, being at [`Layer::FireTarget`](crate::Layer), UNDER the
/// enemy sprite).
pub(super) fn register_fire_target_systems(app: &mut App) {
    app.add_systems(
        Update,
        draw_fire_target
            .in_set(PresenterSystems::Overlay)
            .run_if(resource_exists::<BattleInProgress>),
    );
}

/// Registers the GTW-545 area-damage-field VIEW: the persistent per-cell hazard-wash overlay
/// DRAW system, into the [`PresenterSystems::Overlay`] stage. (The transient
/// per-tick FCT `"-N"` reader moved into the GTW-572 consequence palette —
/// [`register_consequence_fct_families`](super::fx::register_consequence_fct_families) registers
/// the `FieldFct` family; the old presenter-side idempotent `add_message::<FieldTicked>` is GONE
/// (GTW-572 C4): the sim's acts plugin registers the buffer in a live battle, and a
/// presenter-only harness without it keeps the family reader inert.)
///
/// The presenter reads the AUTHORITATIVE sim [`FieldRegistry`](gdtf_battle_sim::FieldRegistry)
/// resource DIRECTLY (a battle-lifetime resource `setup_battle` seeds from the situation's
/// authored `fields:` list and the GTW-547 spawn API grows) and DRAWS one translucent hazard
/// tile per fielded cell — the one-way `input → presenter → sim` direction (the presenter reads
/// sim truth and draws it; it never writes the sim). This is the SAME one-way sim-read shape as
/// the terrain draw reading the cover ledger.
///
/// Unlike the DEBUG-only reachable overlay, this is a SHIPPING view (the playability rule: a
/// damage zone MUST be visible AND its per-turn drain MUST show), so it is NOT
/// `#[cfg(debug_assertions)]`-gated.
///
/// It wires:
///
/// - [`draw_field_overlay`] — the persistent per-cell hazard wash. It draws each fielded cell on
///   the active storey with a pooled, mutated-in-place [`Sprite`] (never despawn-respawned), tinted
///   per the field's [`DamageType`](gdtf_battle_sim::DamageType), hard-cut to the active storey.
///   The overlay follows `PageUp` with NO extra wiring: it reads `Res<ActiveLevel>` live every
///   frame. Gated `run_if(resource_exists::<FieldRegistry>)` — the sim's live-field witness
///   (`setup_battle` inserts it, teardown removes it), so it stays inert when no battle has seeded
///   a field registry (a `Res<FieldRegistry>` param panics validation without the resource,
///   `bevy-traps.md` #1). It needs NO render resource (solid-tint sprites, not atlas tiles) and the
///   always-present `init_resource`-d [`ActiveLevel`](crate::ActiveLevel). Its `Overlay` stage
///   membership (chained after `Compose` — GTW-623) makes the hazard wash composite OVER the
///   fogged battlefield, consistent with the reachable / path-preview placement.
pub(super) fn register_field_overlay_systems(app: &mut App) {
    app.add_systems(
        Update,
        draw_field_overlay
            .in_set(PresenterSystems::Overlay)
            .run_if(resource_exists::<FieldRegistry>),
    );
}

/// Registers the GTW-387 / GTW-450 reachable-range DEBUG overlay: the [`ReachableCells`]
/// read-seam, the [`ReachableOverlayEnabled`] flag (seeded from the env var), and the DRAW
/// system into the [`PresenterSystems::Overlay`] stage.
///
/// DEBUG-ONLY (GTW-450 C1): this whole fn — and every item it names — compiles only under
/// `#[cfg(debug_assertions)]`. A release build excludes it, so the overlay never renders
/// in shipping play (it washes the FOV green — visual noise the user ruled out).
///
/// The PRESENTER owns the [`ReachableCells`] read-seam (`init_resource`-d here) plus this
/// draw system; the INPUT crate POPULATES the resource by calling
/// [`reachable_within`](gdtf_battle_sim::reachable_within) for the selected ganger — the
/// [`PathPreview`](crate::PathPreview) precedent, where the presenter DEFINES the type
/// and input WRITES it, keeping the `input → presenter → sim` direction.
///
/// [`draw_reachable_overlay`] draws each reachable cell on the active storey with a
/// pooled, mutated-in-place [`Sprite`] (never despawn-respawned), hard-cut to the active
/// storey. The overlay follows `PageUp` with NO extra wiring: it reads `Res<ActiveLevel>`
/// live every frame.
///
/// The [`ReachableOverlayEnabled`] flag is inserted ONCE here from
/// [`ReachableOverlayEnabled::from_env`] (reading
/// [`REACHABLE_OVERLAY_ENV`](crate::REACHABLE_OVERLAY_ENV) —
/// `GDTF_DEBUG_REACHABLE_OVERLAY`); UNSET → `false` → the draw system's `run_if` is false
/// → NO overlay renders by default (GTW-450 C3 / C4).
///
/// Gated `run_if(resource_exists::<BattleInProgress>)` — the sim's live-battle witness —
/// AND `resource_exists::<SquadVisibility>` so the resource is present (inserted by the
/// sim's `setup_battle`) AND the [`ReachableOverlayEnabled`] flag VALUE (the C3 runtime
/// opt-in; the flag is always present here, so the gate reads its value, not its
/// existence). It needs NO render resource (solid-tint sprites, not atlas tiles) and the
/// always-present `init_resource`-d [`ReachableCells`] and [`ActiveLevel`](crate::ActiveLevel).
/// Its `Overlay` stage membership (chained after `Compose` — GTW-623) makes the range tint
/// composite OVER the fogged battlefield, consistent with the path-preview placement.
#[cfg(debug_assertions)]
pub(super) fn register_reachable_overlay_systems(app: &mut App) {
    // GTW-450 C3 — read the env var ONCE at startup (NOT per-frame) into the flag resource.
    app.insert_resource(ReachableOverlayEnabled::from_env())
        // GTW-387 — the reachable-range overlay read-seam (the cells the selected ganger can
        // reach within its remaining TU). Its Default is the empty set; the input crate
        // POPULATES it (clearing it when no ganger is selected).
        .init_resource::<ReachableCells>()
        .add_systems(
            Update,
            draw_reachable_overlay
                .in_set(PresenterSystems::Overlay)
                .run_if(
                    resource_exists::<BattleInProgress>
                        .and_then(resource_exists::<SquadVisibility>)
                        // GTW-450 C3 — the runtime opt-in: render only when the flag is true.
                        .and_then(reachable_overlay_enabled),
                ),
        );
}

/// Run-condition: whether the reachable-range DEBUG overlay is enabled this process
/// (GTW-450 C3) — reads the [`ReachableOverlayEnabled`] flag VALUE. DEBUG-only.
///
/// `Option<Res<…>>` (fail-closed if absent: the focused presenter `build` always inserts
/// it, but a harness might not) so the draw/populate systems stay inert unless the flag is
/// present AND `true`.
#[cfg(debug_assertions)]
fn reachable_overlay_enabled(flag: Option<Res<ReachableOverlayEnabled>>) -> bool {
    flag.is_some_and(|flag| **flag)
}
