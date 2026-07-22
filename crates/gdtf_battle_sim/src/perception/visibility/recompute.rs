//! The ONE squad-fog writer system [`recompute_visibility`] and its trigger gate
//! [`should_recompute_visibility`] (GTW-341, leaf 5 of the GTW-13 FOV epic;
//! `docs/combat/visibility.md`).
//!
//! [`recompute_visibility`] is the **sole mutator** of
//! [`SquadVisibility`](crate::visibility::SquadVisibility): it gathers every conscious
//! player-faction observer, calls GTW-340's [`union_fov`](crate::visibility::union_fov)
//! then [`accrue`](crate::visibility::accrue), and writes the resource back. Every
//! downstream consumer (the GTW-11 fog gate, GTW-70 AI, GTW-38 reaction fire) is
//! READ-ONLY — they read the pure accessors, never write.
//!
//! **Ambush invariant** (`docs/combat/visibility.md` §"Rendered-only planning"): each
//! accepted move step writes the mover's new [`Position`], which trips
//! `Changed<Position>` and re-runs THIS system. So an ambusher is revealed **iff the
//! mover's own sight reaches it from where it stopped** — the per-trigger recompute IS
//! the reveal mechanic; there is NO separate per-step reveal system.

use bevy::prelude::{
    Changed, Entity, MessageReader, Or, Query, RemovedComponents, Res, ResMut, With,
};

use crate::{
    battle::{BattleReady, PlayerFaction},
    cover::CoverLedger,
    ganger::{Facing, Faction, LifeState, Position, Stance},
    los::PeekOffset,
    occupancy::{OccupancyGrid, VisionOccluderChanged},
    occupancy_sync::{CoverDestroyed, SlabDestroyed},
    surface::SurfaceGrid,
    terrain::entity::{BlocksVision, TerrainCell},
    tuning::CombatTuning,
    visibility::{FovObserver, SquadVisibility, accrue, union_fov},
};

/// The change-detection filter the trigger gate and the writer observe: a ganger that
/// **moved** (`Changed<`[`Position`]`>`), **re-posed** (`Changed<`[`Stance`]`>`), or
/// **flipped life state** (`Changed<`[`LifeState`]`>`). Mirrors the `occupancy_sync`
/// [`MovedOrReposed`](crate::occupancy_sync) precedent, widened with the life flip
/// (clause 2 — a downed observer drops its FOV on the next recompute).
type MovedReposedOrFlipped = Or<(
    Changed<Position>,
    Changed<Stance>,
    Changed<LifeState>,
    Changed<PeekOffset>,
)>;

/// Whether the squad fog must be recomputed THIS update — the trigger gate (clause 2).
///
/// Returns `true` when ANY of the events that can change what the squad sees occurred:
///
/// * a **player-faction observer** moved / re-posed / flipped life state — some entity
///   matched `MovedReposedOrFlipped` AND belongs to the [`PlayerFaction`] (a non-player
///   ganger's move never re-reveals the squad's own fog, so it is filtered out here);
/// * a [`CoverDestroyed`] message was buffered this update (a smashed wall can open a
///   sightline);
/// * a [`SlabDestroyed`] message was buffered this update (GTW-365 — a smashed floor/roof
///   slab opens a vertical sightline through the hole: the round + LOS march already fly
///   through a `Destroyed` slab, so the recompute reflects the reopened line);
/// * a tag-derived VISION occluder changed this update (GTW-502 C6) — a terrain entity
///   GAINED or RETUNED a [`BlocksVision`](crate::terrain::entity::BlocksVision) component
///   (`Or<(Added, Changed)>`) OR LOST one (`RemovedComponents<BlocksVision>`). Adding /
///   retuning / removing an occluder changes the [`VisionBlocking`](crate::occupancy::VisionBlocking)
///   surface the LoS/FoV march reads, so the precomputed squad FOV must re-fire (the
///   cover/slab-destroyed mirror, but driven by the component itself rather than a message —
///   the occluder has no destruction message of its own); or
/// * a [`BattleReady`] message was buffered this update (the setup-spawn FOV — the first
///   recompute that fills the freshly-inserted [`SquadVisibility`]).
///
/// All [`MessageReader`]s AND the [`RemovedComponents`] reader are **drained fully**
/// ([`count`](Iterator::count)`() > 0`) so a signal read here is consumed and cannot re-fire
/// the gate on a later update (clause 2 — no stale re-fire). `RemovedComponents` MUST be
/// drained every run (`bevy-traps.md` #4) — a `run_if` predicate is evaluated each tick the
/// set is checked, so reading it here keeps its cursor advancing. This run-condition reads
/// the signals with its OWN reader cursor, independent of [`recompute_visibility`]'s `Changed`
/// re-evaluation: the writer never reads these buffers, it only recomputes off the live world,
/// so draining here is safe.
///
/// A `run_if` gate (not a body early-return) so the writer's expensive
/// [`union_fov`](crate::visibility::union_fov) candidate scan is skipped entirely on a
/// quiet update (`bevy-traps.md` #3 — explicit gating).
///
/// [`PlayerFaction`] is read as `Option<Res<_>>` so the gate is panic-free even outside a
/// live battle — the writer it gates runs in the
/// [`SimSystems::Simulate`](crate::occupancy_sync::SimSystems) band (itself gated on
/// [`BattleInProgress`](crate::battle::BattleInProgress)), but a run-condition is still
/// evaluated when the set is checked, so its params must be present-or-absent safe
/// (`bevy-traps.md` #1). Absent player faction = no battle = no recompute.
#[must_use]
pub fn should_recompute_visibility(
    moved: Query<&Faction, MovedReposedOrFlipped>,
    player: Option<Res<PlayerFaction>>,
    occluder_changed: Query<(), (With<TerrainCell>, VisionOccluderChanged)>,
    mut occluder_removed: RemovedComponents<BlocksVision>,
    mut cover_destroyed: MessageReader<CoverDestroyed>,
    mut slab_destroyed: MessageReader<SlabDestroyed>,
    mut ready: MessageReader<BattleReady>,
) -> bool {
    // Drain ALL buffers fully every update — a count, not a take-first — so a buffered
    // message is consumed here and never re-fires the gate next update (clause 2).
    let cover_changed = cover_destroyed.read().count() > 0;
    // GTW-365: a destroyed slab opens a sightline through the hole (the LOS march flies
    // through a Destroyed slab the same way the round does), so a buffered SlabDestroyed
    // re-fires the recompute — the slab mirror of the cover trigger.
    let slab_changed = slab_destroyed.read().count() > 0;
    let battle_ready = ready.read().count() > 0;
    // GTW-502 C6: a tag-derived VISION occluder was added/retuned (Added|Changed) OR removed
    // (RemovedComponents) this tick — the VisionBlocking surface the LoS/FoV march reads
    // changed, so the squad FOV must re-fire. RemovedComponents is drained EVERY run
    // (`.read().count()`, bevy-traps.md #4) so its cursor advances and a removal cannot
    // re-fire next tick. Added implies Changed, so the query's Or is read as "added-or-retuned".
    let occluder_added_or_retuned = !occluder_changed.is_empty();
    let occluder_removed = occluder_removed.read().count() > 0;
    let vision_occluder_changed = occluder_added_or_retuned || occluder_removed;
    // A player-faction observer moved / re-posed / flipped: only the player's own gangers
    // feed the squad fog, so an enemy move is not a recompute trigger (clause 1 / 2). With
    // no player faction (no live battle) there is no squad to re-reveal.
    let player_observer_changed = player
        .as_deref()
        .is_some_and(|player| moved.iter().any(|faction| *faction == **player));
    player_observer_changed
        || cover_changed
        || slab_changed
        || battle_ready
        || vision_occluder_changed
}

/// The ONE writer of [`SquadVisibility`] — recompute the squad fog from every conscious
/// player-faction observer's FOV (GTW-341 clause 1; `docs/combat/visibility.md`).
///
/// Gathers each player-faction ganger's borrow-view ([`FovObserver`]), feeds them to
/// GTW-340's [`union_fov`](crate::visibility::union_fov) (the squad VISIBLE union over the
/// disc-bounded authored/occupied candidate set, banding each candidate the shot-pipeline
/// way) and then [`accrue`](crate::visibility::accrue) (VISIBLE replaces, EXPLORED grows
/// monotonically), and writes the result back into the [`SquadVisibility`] resource. This
/// is the SOLE mutator: every consumer reads the pure accessors and never writes (clause 1).
///
/// The observer set is restricted to the [`PlayerFaction`] here (only the player's side
/// has a squad fog); [`union_fov`](crate::visibility::union_fov) further restricts to the
/// CONSCIOUS (`LifeState::is_active`) ones, so a Downed / Dead player ganger contributes
/// no FOV (clause 1 — the life-flip trigger drops its sight on the next recompute).
///
/// The `is_dead` corpse predicate is composed off the same observer [`Query`] (a separate
/// READ-ONLY borrow): a `Dead` ganger is a corpse the LOS march flies THROUGH (the GTW-317
/// pass-through-corpse rule [`can_see`](crate::los::can_see) / [`has_los`](crate::los::has_los) take). An entity
/// absent from the query (cover, surface) is never a corpse.
///
/// Param-only (`Query` / `Res` / `ResMut`) — no `&mut World`, no `Commands::spawn`
/// (`bevy-traps.md` #7). It reads the four battle-lifetime grids/tuning as `Res<_>`:
/// the system is gated on
/// [`resource_exists`](bevy::prelude::resource_exists)`::<`[`SquadVisibility`]`>` (which
/// shares the [`BattleInProgress`](crate::battle::BattleInProgress) lifetime), so those
/// reads are panic-free (`bevy-traps.md` #1).
pub fn recompute_visibility(
    observers: Query<(&Position, &Stance, &Facing, &LifeState, &Faction)>,
    occupancy: Res<OccupancyGrid>,
    surface: Res<SurfaceGrid>,
    cover: Res<CoverLedger>,
    tuning: Res<CombatTuning>,
    player: Res<PlayerFaction>,
    mut squad: ResMut<SquadVisibility>,
) {
    // The corpse predicate (clause 1): a `Dead` ganger does not block sight. A READ-ONLY
    // borrow of the same observer query — the LOS march passes through it (GTW-317).
    let is_dead = |entity: Entity| {
        observers
            .get(entity)
            .is_ok_and(|(.., life, _)| *life == LifeState::Dead)
    };

    // Assemble the player-faction observers' borrow-views. union_fov skips the
    // inactive (Downed / Dead) ones, so the conscious filter is split: faction here,
    // life inside union_fov (clause 1).
    // GTW-390: look up each observer's authored stair-tile eye-lift from the
    // OccupancyGrid (stair_eye_offset_at returns 0.0 for non-stair cells). The stance
    // gate (Prone → 0.0) is applied inside eye_anchor, not here.
    let fov_observers: Vec<FovObserver> = observers
        .iter()
        .filter(|(.., faction)| **faction == **player)
        .map(|(position, stance, facing, life, _)| FovObserver {
            position,
            stance,
            facing,
            life: *life,
            stair_eye_offset: occupancy.stair_eye_offset_at(position),
        })
        .collect();

    // Recompute the squad VISIBLE union, then accrue it into the new fog (VISIBLE
    // replaces, EXPLORED grows). Replace the resource in place — the SOLE write.
    let visible_next = union_fov(
        &fov_observers,
        &occupancy,
        &surface,
        &cover,
        &tuning,
        is_dead,
    );
    *squad = accrue(&squad, visible_next);
}

#[cfg(test)]
mod test {
    use bevy::{
        ecs::system::RunSystemOnce as _,
        prelude::{Entity, Messages, World},
    };

    use crate::{
        battle::{BattleReady, PlayerFaction},
        ganger::{Faction, Position, Stance, StanceKind},
        los::PeekOffset,
        metric::{Cell, CellLevel, Level},
        occupancy_sync::{CoverDestroyed, SlabDestroyed},
        visibility::should_recompute_visibility,
    };

    /// Gang `0` is the player; gang `1` is the enemy.
    const PLAYER: Faction = Faction::new(0);
    /// The enemy gang.
    const ENEMY: Faction = Faction::new(1);

    /// A `(cell, level)` key on the ground floor — the spawn anchor for test gangers.
    fn ground(x: i32, y: i32) -> CellLevel {
        CellLevel::new(Cell::new(x, y), Level::new(0))
    }

    /// Evaluate the trigger predicate against `world` ONCE — run the run-condition AS a
    /// system in isolation (bevy-traps.md #7 carve-out (b): a pure-sim unit test with no
    /// `App`). Registers the two message buffers the reader params need so param
    /// validation passes, then runs `should_recompute_visibility` once.
    fn run_predicate(world: &mut World) -> bool {
        world.init_resource::<Messages<CoverDestroyed>>();
        world.init_resource::<Messages<SlabDestroyed>>();
        world.init_resource::<Messages<BattleReady>>();
        // RunSystemOnce returns the system's `bool` output (Err only on a param-validation
        // failure, which cannot happen here — the buffers are init'd, the query + Option<Res>
        // never fail). Default to `false` on the impossible Err so the test stays panic-free.
        world
            .run_system_once(should_recompute_visibility)
            .unwrap_or(false)
    }

    #[test]
    fn battle_ready_message_fires_the_gate() {
        // The setup-spawn FOV trigger: a BattleReady buffered this update fires the
        // gate even with no observer change and no player faction wired yet.
        let mut world = World::new();
        world.init_resource::<Messages<CoverDestroyed>>();
        world.init_resource::<Messages<SlabDestroyed>>();
        world.init_resource::<Messages<BattleReady>>();
        world
            .resource_mut::<Messages<BattleReady>>()
            .write(BattleReady);
        assert!(
            run_predicate(&mut world),
            "a buffered BattleReady must fire the recompute gate (spawn-FOV trigger)"
        );
    }

    #[test]
    fn cover_destroyed_message_fires_the_gate() {
        // A smashed wall can open a sightline — a CoverDestroyed fires the gate.
        let mut world = World::new();
        world.init_resource::<Messages<CoverDestroyed>>();
        world.init_resource::<Messages<SlabDestroyed>>();
        world.init_resource::<Messages<BattleReady>>();
        world
            .resource_mut::<Messages<CoverDestroyed>>()
            .write(CoverDestroyed::new(ground(3, 3)));
        assert!(
            run_predicate(&mut world),
            "a buffered CoverDestroyed must fire the recompute gate"
        );
    }

    #[test]
    fn slab_destroyed_message_fires_the_gate() {
        // GTW-365: a smashed floor/roof slab opens a vertical sightline through the hole
        // — a buffered SlabDestroyed fires the recompute gate (the slab mirror of the
        // cover trigger). Mechanism, not a magnitude.
        let mut world = World::new();
        world.init_resource::<Messages<CoverDestroyed>>();
        world.init_resource::<Messages<SlabDestroyed>>();
        world.init_resource::<Messages<BattleReady>>();
        world
            .resource_mut::<Messages<SlabDestroyed>>()
            .write(SlabDestroyed::new(ground(3, 3)));
        assert!(
            run_predicate(&mut world),
            "a buffered SlabDestroyed must fire the recompute gate (the opened-hole sightline)"
        );
    }

    #[test]
    fn player_observer_move_fires_the_gate() {
        // A freshly-spawned player ganger reads as Changed<Position> on the first
        // evaluation (Bevy first-run change semantics) — a player observer changed.
        let mut world = World::new();
        world.insert_resource(PlayerFaction::new(PLAYER));
        world.spawn((
            Position::new(ground(5, 5)),
            Stance::new(StanceKind::Standing),
            PLAYER,
        ));
        assert!(
            run_predicate(&mut world),
            "a player-faction observer that moved/posed must fire the gate"
        );
    }

    #[test]
    fn enemy_only_move_does_not_fire_the_gate() {
        // Only the PLAYER side feeds the squad fog: an enemy-only change is NOT a
        // trigger (clause 1 / 2 — the faction filter in the predicate).
        let mut world = World::new();
        world.insert_resource(PlayerFaction::new(PLAYER));
        world.spawn((
            Position::new(ground(9, 9)),
            Stance::new(StanceKind::Standing),
            ENEMY,
        ));
        assert!(
            !run_predicate(&mut world),
            "an enemy-only move must NOT fire the recompute gate"
        );
    }

    #[test]
    fn quiet_update_does_not_fire_the_gate() {
        // No messages, no player faction, no observer: the gate stays closed.
        let mut world = World::new();
        let _no_observer: Entity = world.spawn_empty().id();
        assert!(
            !run_predicate(&mut world),
            "a quiet update with no trigger must NOT fire the recompute gate"
        );
    }

    #[test]
    fn missing_player_faction_with_observer_does_not_fire() {
        // The PlayerFaction is read as Option<Res>: absent (no live battle) => no
        // squad to re-reveal even if some entity changed (panic-free gate).
        let mut world = World::new();
        world.spawn((
            Position::new(ground(5, 5)),
            Stance::new(StanceKind::Standing),
            PLAYER,
        ));
        assert!(
            !run_predicate(&mut world),
            "with no PlayerFaction (no battle) the gate must stay closed, not panic"
        );
    }

    /// A player-faction ganger with a freshly-set [`PeekOffset`] fires the gate
    /// (GTW-393 C3 trigger: `Changed<PeekOffset>` is in the `Or<(…)>` alias).
    ///
    /// A freshly-spawned ganger reads as `Changed<PeekOffset>` on the first evaluation
    /// (Bevy first-run change semantics — the same as `Changed<Position>` in the
    /// `player_observer_move_fires_the_gate` test). Because the ganger belongs to the
    /// PLAYER faction, the gate returns `true`.
    #[test]
    fn player_peek_change_fires_the_gate() {
        let mut world = World::new();
        world.insert_resource(PlayerFaction::new(PLAYER));
        world.spawn((
            Position::new(ground(5, 5)),
            Stance::new(StanceKind::Standing),
            PeekOffset::default(),
            PLAYER,
        ));
        assert!(
            run_predicate(&mut world),
            "a player-faction observer with a Changed<PeekOffset> must fire the gate              (GTW-393 C3 trigger)"
        );
    }

    /// An enemy-faction ganger with a `PeekOffset` does NOT fire the gate — the
    /// faction filter in the trigger query covers the new `Or` arm (GTW-393 C3 + C4:
    /// only player observers feed the squad fog; an enemy peek never recomputes it).
    #[test]
    fn enemy_peek_change_does_not_fire() {
        let mut world = World::new();
        world.insert_resource(PlayerFaction::new(PLAYER));
        world.spawn((
            Position::new(ground(9, 9)),
            Stance::new(StanceKind::Standing),
            PeekOffset::default(),
            ENEMY,
        ));
        assert!(
            !run_predicate(&mut world),
            "an enemy-faction observer with a Changed<PeekOffset> must NOT fire the gate              (only player observers feed the squad fog — GTW-393 C3 faction filter)"
        );
    }
}
