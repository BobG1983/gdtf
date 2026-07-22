//! The ONE cross-level-signals DERIVE system (GTW-596): reads the sim through the
//! SAME pure fog queries [`present_fog`](crate::present_fog) /
//! [`resolve_ganger_visibility`](crate::resolve_ganger_visibility) use, then
//! aggregates + caps into the [`CrossLevelSignals`] resource.

use bevy::{ecs::system::SystemParam, platform::collections::HashSet, prelude::*};
use gdtf_battle_sim::{
    battle::PlayerFaction,
    prelude::{Cell, Faction, Level, LifeState, OccupancyGrid, Position},
    surface::SurfaceGrid,
    vertical::VerticalLinkGraph,
    visibility::SquadVisibility,
};

use super::{
    aggregate::build_signals, connector::gather_connectors, drop_depth::gather_drop_depth,
    threat::gather_threats, types::CrossLevelSignals,
};
use crate::{ActiveLevel, TerrainSprite};

/// The derive system's resource/query bundle (`bevy-traps.md` #7 / the
/// `StaticMap` precedent) — keeps [`derive_cross_level_signals`]'s signature
/// under the `too_many_arguments` lint. Every field mirrors the same
/// fail-closed shape the system doc describes: [`SquadVisibility`] /
/// [`PlayerFaction`] / the terrain resources are each `Option<Res<_>>` so a
/// focused harness missing one never panics (`bevy-traps.md` #1).
#[derive(SystemParam)]
pub struct CrossLevelSimFacts<'w, 's> {
    /// The presenter-owned active view storey.
    active:    Res<'w, ActiveLevel>,
    /// The sim's squad fog sets — absent (a focused harness) fails the whole
    /// derive closed (see [`derive_cross_level_signals`]).
    squad:     Option<Res<'w, SquadVisibility>>,
    /// The battle's player faction — absent treats every ganger as non-player
    /// (the `gather_threats` fail-closed default).
    player:    Option<Res<'w, PlayerFaction>>,
    /// Every ganger's position / faction / life state — the Threat scan's input.
    gangers:   Query<'w, 's, (&'static Position, &'static Faction, &'static LifeState)>,
    /// The persistent slab-existence grid — the `DropDepth` scan's input.
    surface:   Option<Res<'w, SurfaceGrid>>,
    /// The static-terrain grid — the `DropDepth` scan's walkability check.
    occupancy: Option<Res<'w, OccupancyGrid>>,
    /// The validated vertical-link index — the `ConnectorDelta` scan's input.
    graph:     Option<Res<'w, VerticalLinkGraph>>,
    /// The drawn terrain-tile markers — the `DropDepth` gather's bounded
    /// "built footprint" proxy (see `drop_depth.rs`).
    terrain:   Query<'w, 's, &'static TerrainSprite>,
}

/// `Update` ([`PresenterSystems::Compose`](crate::PresenterSystems) — the SAME
/// composition stage `present_fog` / [`resolve_ganger_visibility`](crate::resolve_ganger_visibility)
/// occupy, chained strictly after `Scene`): derive [`CrossLevelSignals`] for the
/// CURRENT active storey.
///
/// Composes THREE independent gathers, exactly mirroring the ganger-visibility
/// resolver's own composition shape (a fog fact + the settled scene, never a
/// parallel visibility check):
///
/// - [`gather_threats`] — the fog-gated cross-level enemy scan, through the SAME
///   [`is_ganger_visible`](gdtf_battle_sim::visibility::is_ganger_visible) free fn
///   [`present_fog`](crate::present_fog) / the ganger-visibility resolver read;
/// - [`gather_drop_depth`] — the hole/ledge scan over the live [`SurfaceGrid`] /
///   [`OccupancyGrid`], fog-gated on squad-EXPLORED (the terrain-draw treatment);
/// - [`gather_connectors`] — one badge per on-storey [`VerticalLinkGraph`]
///   endpoint, same EXPLORED gate.
///
/// [`build_signals`] then dedupes + caps per cell (the RESOLVED SPEC), and the
/// result is written through
/// [`ResMut::set_if_neq`](bevy::prelude::DetectChangesMut::set_if_neq) so the
/// resource is change-tick-quiet: the draw system re-walks its pool when this
/// resource's `is_changed()` reads true (see
/// [`draw_cross_level_signals`](super::draw::draw_cross_level_signals) for the
/// second, independent [`ActiveLevel`] trigger it ALSO honours).
///
/// **Fail-closed:** with no [`SquadVisibility`] resident (a focused harness that
/// opens `BattleInProgress` without the full `setup_battle`) this clears the
/// signal set entirely rather than guessing — never leak a signal without the fog
/// facts to gate it. The terrain resources ([`SurfaceGrid`] / [`OccupancyGrid`] /
/// [`VerticalLinkGraph`]) are each `Option<Res<_>>` for the same reason
/// (`bevy-traps.md` #1): absent, they simply contribute no `DropDepth` /
/// `ConnectorDelta` candidates this frame.
///
/// Param-only (`bevy-traps.md` #7): the [`ResMut<CrossLevelSignals>`] write plus
/// the [`CrossLevelSimFacts`] bundle (the [`StaticMap`](crate::StaticMap)
/// precedent keeping the signature under the `too_many_arguments` lint).
/// Battle-gated by [`TopDownRendererPlugin`](crate::TopDownRendererPlugin)'s
/// registrar.
pub fn derive_cross_level_signals(
    mut signals: ResMut<CrossLevelSignals>,
    facts: CrossLevelSimFacts,
) {
    let Some(squad) = facts.squad.as_deref() else {
        signals.set_if_neq(CrossLevelSignals::default());
        return;
    };
    let active_level: Level = **facts.active;

    let threats = gather_threats(
        facts.gangers.iter(),
        squad,
        facts.player.as_deref().copied(),
        active_level,
    );

    // DropDepth and ConnectorDelta are INDEPENDENT of each other — each contributes
    // whenever its OWN resources are resident, regardless of whether the other's are
    // (a focused harness may seed a `VerticalLinkGraph` without a `SurfaceGrid`, or
    // vice versa).
    let drops = match (facts.surface.as_deref(), facts.occupancy.as_deref()) {
        (Some(surface), Some(occupancy)) => {
            let drawn = drawn_cells_on(active_level, &facts.terrain);
            gather_drop_depth(active_level, surface, occupancy, &drawn, squad)
        }
        _ => Vec::new(),
    };
    let connectors = match facts.graph.as_deref() {
        Some(graph) => gather_connectors(active_level, graph, squad),
        None => Vec::new(),
    };

    signals.set_if_neq(build_signals(threats, drops, connectors));
}

/// The cells with a DRAWN [`TerrainSprite`] on `active_level` — the drop-depth
/// gather's bounded "built footprint" proxy (see `drop_depth.rs`).
fn drawn_cells_on(active_level: Level, terrain: &Query<&TerrainSprite>) -> HashSet<Cell> {
    terrain
        .iter()
        .filter(|sprite| sprite.at.level() == active_level)
        .map(|sprite| sprite.at.cell())
        .collect()
}
