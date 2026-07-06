//! Squad-fog writer registration — the final-writer ordering contract over every
//! drawn tile / actor sprite, expressed as STAGE MEMBERSHIP.

use bevy::prelude::*;
use gdtf_battle_sim::{BattleInProgress, CombatTuning, SquadVisibility};

use crate::{PresenterSystems, TerrainFogMaterial, present_fog};

/// Registers the GTW-342 squad fog WRITER into the [`PresenterSystems::Compose`] stage.
///
/// [`present_fog`] is the VIEW arm of the squad fog (`docs/combat/visibility.md`): the sim
/// owns the three [`SquadVisibility`](gdtf_battle_sim::SquadVisibility) states and the
/// [`recompute_visibility`](gdtf_battle_sim::recompute_visibility) writer (GTW-341); this
/// presenter READS them and modulates the already-drawn layer in place (the rendered layer
/// IS the fog mask — it never repaints from a snapshot).
///
/// # Ordering (the CRITICAL clause, `bevy-traps.md` #3)
///
/// [`PresenterSystems::Compose`] is chained strictly after [`PresenterSystems::Scene`]
/// (configured ONCE in `TopDownRendererPlugin::build` — GTW-623), so the fog always
/// colours LIVE, freshly-spawned / just-swapped [`TerrainSprite`](crate::TerrainSprite)
/// entities — including after an [`ActiveLevel`](crate::ActiveLevel) cycle, which despawns
/// and respawns the terrain on the SAME update — and runs after the ganger storey-filter
/// writers, making it the SINGLE FINAL writer of each actor sprite's [`Visibility`]: it
/// composes the slice's storey fact AND the fog fact rather than crossing the slice's
/// writer (`docs/combat/visibility.md` §"Composition with the view slice"). This STAGE
/// MEMBERSHIP replaced the old hand-maintained seven-edge `.after` wall (the terrain draw,
/// the two destruction swaps, the emplacement swap, and the three ganger writers — a wall
/// every NEW scene writer had to remember to append itself to, GTW-543 being the last):
/// a new scene writer now joins `Scene` and the fog orders after it with ZERO new edges.
///
/// # Gating (`bevy-traps.md` #1)
///
/// `run_if(resource_exists::<BattleInProgress>)` (the live-battle witness) AND
/// `resource_exists::<SquadVisibility>` (the fog sets — inserted by the sim's `setup_battle`,
/// absent in a focused harness that opens `BattleInProgress` directly; without this guard the
/// `Res<SquadVisibility>` param would panic validation) AND
/// `resource_exists::<Assets<TerrainFogMaterial>>` (GTW-348 — the terrain arm drives each tile's
/// material `saturation` via this store; created by `Material2dPlugin` only when an `AssetServer`
/// is present, so a `MinimalPlugins` app without it never runs the writer) AND
/// `resource_exists::<CombatTuning>` (the `Load`-state combat tuning — the "a real battle's
/// balance data is configured" witness). `GangerSprites` + `ActiveLevel` are `init_resource`-d
/// on build, so they are always present.
///
/// GTW-348 NOTE on the `CombatTuning` gate: the EXPLORED treatment no longer reads `explored_dim`
/// (EXPLORED is full-brightness greyscale, not a brightness dim) and `present_fog` no longer takes
/// a `CombatTuning` param — so the gate is no longer a "the param needs this resource" guard. It
/// is KEPT as the battle-configured witness: in the real app `CombatTuning` is always present
/// during a battle (a `Load`-state resource), so fog runs exactly as before; but a focused
/// presenter harness that drives a bare ganger spawn WITHOUT inserting `CombatTuning` (e.g. the
/// storey-filter `ganger_draw` tests) keeps fog INERT, so this change does not silently flip the
/// actor-fog hard-cut on in those harnesses — GTW-348 is a TERRAIN-only change, so the actor arm's
/// observable behavior is held identical to pre-GTW-348 in every harness.
pub(super) fn register_fog_systems(app: &mut App) {
    app.add_systems(
        Update,
        present_fog.in_set(PresenterSystems::Compose).run_if(
            resource_exists::<BattleInProgress>
                .and_then(resource_exists::<SquadVisibility>)
                .and_then(resource_exists::<Assets<TerrainFogMaterial>>)
                .and_then(resource_exists::<CombatTuning>),
        ),
    );
}
