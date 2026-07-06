//! Squad-fog terrain-writer registration — the fog-composition ordering contract over
//! every drawn tile, expressed as STAGE MEMBERSHIP.

use bevy::prelude::*;
use gdtf_battle_sim::{BattleInProgress, SquadVisibility};

use crate::{PresenterSystems, TerrainFogMaterial, present_fog};

/// Registers the GTW-342 squad fog TERRAIN writer into the
/// [`PresenterSystems::Compose`] stage.
///
/// [`present_fog`] is the terrain VIEW arm of the squad fog
/// (`docs/combat/visibility.md`): the sim owns the three
/// [`SquadVisibility`](gdtf_battle_sim::SquadVisibility) states and the
/// [`recompute_visibility`](gdtf_battle_sim::recompute_visibility) writer (GTW-341); this
/// presenter READS them and modulates the already-drawn layer in place (the rendered layer
/// IS the fog mask — it never repaints from a snapshot). Actor-sprite visibility is the
/// ganger-visibility resolver's
/// ([`resolve_ganger_visibility`](crate::resolve_ganger_visibility), registered by the
/// sibling `gangers` registrar into the same `Compose` stage — GTW-627).
///
/// # Ordering (the CRITICAL clause, `bevy-traps.md` #3)
///
/// [`PresenterSystems::Compose`] is chained strictly after [`PresenterSystems::Scene`]
/// (configured ONCE in `TopDownRendererPlugin::build` — GTW-623), so the fog always
/// colours LIVE, freshly-spawned / just-swapped [`TerrainSprite`](crate::TerrainSprite)
/// entities — including after an [`ActiveLevel`](crate::ActiveLevel) cycle, which despawns
/// and respawns the terrain on the SAME update. This STAGE MEMBERSHIP replaced the old
/// hand-maintained seven-edge `.after` wall: a new scene writer joins `Scene` and the fog
/// orders after it with ZERO new edges.
///
/// # Gating (`bevy-traps.md` #1)
///
/// `run_if(resource_exists::<BattleInProgress>)` (the live-battle witness) AND
/// `resource_exists::<SquadVisibility>` (the fog sets — inserted by the sim's `setup_battle`,
/// absent in a focused harness that opens `BattleInProgress` directly; without this guard the
/// `Res<SquadVisibility>` param would panic validation) AND
/// `resource_exists::<Assets<TerrainFogMaterial>>` (GTW-348 — the terrain arm drives each tile's
/// material `saturation` via this store; created by `Material2dPlugin` only when an `AssetServer`
/// is present, so a `MinimalPlugins` app without it never runs the writer). GTW-627 (C2)
/// DELETED the old `resource_exists::<CombatTuning>` clause: the writer read nothing from
/// it since GTW-348 — it survived only as a pseudo-gate keeping fog inert in focused
/// harnesses, a job now owned by the REAL residency facts above (and, for the actor arm,
/// by the classifier's absent-fog band-only branch).
pub(super) fn register_fog_systems(app: &mut App) {
    app.add_systems(
        Update,
        present_fog.in_set(PresenterSystems::Compose).run_if(
            resource_exists::<BattleInProgress>
                .and_then(resource_exists::<SquadVisibility>)
                .and_then(resource_exists::<Assets<TerrainFogMaterial>>),
        ),
    );
}
