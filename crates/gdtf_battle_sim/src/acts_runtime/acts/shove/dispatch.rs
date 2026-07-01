//! The **shove dispatch** system (GTW-525 C2 / C3 / C6) — [`dispatch_shove`], which drains the
//! buffered [`ShoveRequested`] and, per message, resolves ONE shove through the SHARED verb +
//! apply helper.
//!
//! It handles BOTH shove sources uniformly through the one shared verb
//! ([`resolve_shove`](super::verb::resolve_shove)) + apply helper ([`apply_shove`]):
//!
//! - a [`ShoveSource::Deliberate`] shove (the input-seam `ActIntent::Shove` act, any ganger)
//!   re-gates 8-adjacency + opposing faction + alive and spends the
//!   [`ShoveTu`](crate::tuning::ShoveTu) leaf, then displaces (pure — no wound);
//! - a [`ShoveSource::Weapon`] auto-shove (a connecting melee strike OR ranged shot with the
//!   `shove` weapon tag) was already earned by the connect — no gate re-check, no TU — so it
//!   only displaces.
//!
//! The shove itself draws NO RNG; a resulting FALL routes through the SHARED GTW-523
//! fall-damage fork (in [`apply_shove`]), which draws the two fall streams. Param-only
//! (`bevy-traps.md` #7 — no `&mut World`); fail-closed on missing components / resources.

use bevy::{
    ecs::system::SystemParam,
    prelude::{MessageReader, MessageWriter, Query, Res, ResMut, With},
};

use super::{
    apply::{ShoveFallEnv, ShoveTargetSurfaces, apply_shove},
    verb::resolve_shove,
};
use crate::{
    acts::{
        InjuryInflicted,
        request::{ShoveRequested, ShoveSource},
    },
    armor::{PieceArmorMut, Wears, WornBy},
    downed_acts::is_8_adjacent,
    falls::FallOccurred,
    ganger::{Faction, Hp, LifeState, Luck, Position, Toughness, Tu, Wounds},
    inflicted_wound::InflictedWounds,
    injuries::{InjuryRegistry, InjuryTables},
    occupancy::OccupancyGrid,
    rng::{InjuryRng, SeverityRng},
    surface::SurfaceGrid,
    tu::spend_tu,
    tuning::CombatTuning,
};

/// The ONE ganger query the shove dispatch reads gate snapshots from AND folds the shove/fall
/// onto — the shover's + target's [`Faction`] + the shoved ganger's `&mut Position` + the fall
/// fold's `&mut Hp`/`Wounds`/`LifeState`/`InflictedWounds` + the read §6 stats, factored into a
/// `type` so [`dispatch_shove`] stays under clippy's type-complexity gate.
///
/// ONE query — NOT a read-query + a disjoint write-query — because the gate reads
/// ([`Position`] / [`Faction`] / [`LifeState`]) and the fold writes ([`Position`] / [`Hp`] /
/// [`Wounds`] / [`LifeState`] / [`InflictedWounds`]) OVERLAP on `Position` + `LifeState`; two
/// separate queries (one `&Position`, one `&mut Position`) would `B0001`-conflict inside the
/// SAME system. So the gate reads are taken via `query.get()` (immutable) as `Copy` snapshots
/// and the fold applies via `query.get_mut()` (exclusive) — the [`apply_falls`](crate::falls::apply_falls)
/// single-`FallerQuery` precedent (its predicate reads + its fold writes ride ONE query).
type ShoveGangerQuery<'world, 'state> = Query<
    'world,
    'state,
    (
        &'static mut Position,
        &'static mut Hp,
        &'static mut Wounds,
        &'static mut LifeState,
        &'static mut InflictedWounds,
        &'static Toughness,
        &'static Luck,
        &'static Faction,
    ),
>;

/// The world grids + tuning + injury content the shove + its fall reads, bundled into one
/// [`SystemParam`] so [`dispatch_shove`] stays under Bevy's 16-param limit (the `FallGrids`
/// precedent).
///
/// Every field is a battle-lifetime resource taken `Option<Res<T>>` so a focused harness that
/// opens `BattleInProgress` WITHOUT the full setup flow does not panic this runtime system on a
/// resource's absence (`bevy-traps.md` #1): the persistent [`SurfaceGrid`] (the shove support
/// scan + the fall drop scan), the [`OccupancyGrid`] (the blocked-destination check), the
/// [`CombatTuning`] (the shove TU + the fall magnitude + §6 scaling), and the injury
/// [`InjuryTables`] / [`InjuryRegistry`] pool. With any absent, [`dispatch_shove`] resolves no
/// shove (a safe, defined fallback — never a panic). In the real app all are sim-set at setup.
#[derive(SystemParam)]
pub struct ShoveGrids<'w> {
    /// The persistent surface grid — the shove support check + the fall drop scan.
    surface:   Option<Res<'w, SurfaceGrid>>,
    /// The coarse occupancy grid — the blocked-destination check (a ganger / wall / cover).
    occupancy: Option<Res<'w, OccupancyGrid>>,
    /// The combat tuning — the [`ShoveTu`](crate::tuning::ShoveTu) leaf + the fall magnitude /
    /// §6 severity scaling / wound costs.
    tuning:    Option<Res<'w, CombatTuning>>,
    /// The shared weighted `(part, severity)` injury tables — the fall's §8 roll pool (AS-IS).
    tables:    Option<Res<'w, InjuryTables>>,
    /// The injury registry — resolves the fall's §8 roll key to its authored def.
    registry:  Option<Res<'w, InjuryRegistry>>,
}

/// The two seeded draw streams a shove's FALL advances, taken `Option<ResMut<…>>` so a focused
/// harness that opens `BattleInProgress` WITHOUT the full setup flow does not panic this system
/// on a stream's absence (`bevy-traps.md` #1; the `FallRngs` precedent).
///
/// The shove itself draws NO RNG; these advance ONLY on a shove that triggers a fall (the
/// shared GTW-523 fork). REUSES [`SeverityRng`] + [`InjuryRng`] — no new stream.
#[derive(SystemParam)]
pub struct ShoveRngs<'w> {
    /// The §6 severity-roll stream — one draw per shoved-off-a-ledge faller.
    severity: Option<ResMut<'w, SeverityRng>>,
    /// The §8 injury-roll stream — one draw per such faller with a non-graze / non-fatal wound.
    injury:   Option<ResMut<'w, InjuryRng>>,
}

/// **Dispatch** buffered [`ShoveRequested`] messages — the deliberate SHOVE act AND the
/// weapon-tag auto-shove (GTW-525 C2 / C3 / C6), resolved through the ONE shared shove verb +
/// apply helper.
///
/// For each request:
///
/// 1. **Snapshot** the shover's + target's [`Position`] / [`Faction`] / [`LifeState`] (Copy),
///    releasing the read borrow before the mutation. A missing read component skips
///    (fail-closed).
/// 2. **Gate + charge (deliberate only).** A [`ShoveSource::Deliberate`] shove re-checks
///    [`is_8_adjacent`](crate::downed_acts::is_8_adjacent) + opposing faction + the target
///    [`LifeState::is_active`] (an alive opposing 8-adjacent target), then spends the
///    [`ShoveTu`](crate::tuning::ShoveTu) leaf off the shover's `&mut Tu` (saturating) — the
///    act costs TU whether or not the displacement lands a fall. A [`ShoveSource::Weapon`]
///    auto-shove skips both (the connecting attack already gated + charged).
/// 3. **Resolve** the one-cell displacement away from the shover via the SHARED
///    [`resolve_shove`](super::verb::resolve_shove) — supported=>move, unsupported=>fall (via
///    the shared [`resolve_drop`](crate::falls::resolve_drop)), blocked=>no-op. NO RNG.
/// 4. **Apply** via the SHARED [`apply_shove`]: rewrite the target's [`Position`], and on a
///    fall run the shared GTW-523 fall-damage fork (the ONLY RNG a shove touches) + emit
///    [`FallOccurred`] (+ any [`InjuryInflicted`]).
///
/// Fail-closed: a missing grid / stream / component simply resolves no shove (never a panic).
/// The shove itself is deterministic + RNG-free; the fall draws the two GTW-523 streams, so the
/// same seed + same message order yields identical outcomes. Param-only (`bevy-traps.md` #7).
#[expect(
    clippy::too_many_arguments,
    reason = "the shove dispatch threads the request reader, the single ganger query (read + \
              fold), the shover-Tu query, the two armor relationship queries, the grouped grids \
              (ShoveGrids) + fall streams (ShoveRngs) bundles, and the FallOccurred + \
              InjuryInflicted writers — the irreducible access set (the apply_falls / \
              dispatch_melee argument-count precedent)"
)]
pub fn dispatch_shove(
    mut requests: MessageReader<ShoveRequested>,
    mut gangers: ShoveGangerQuery,
    mut tu_q: Query<&mut Tu>,
    wears: Query<&Wears>,
    mut pieces: Query<PieceArmorMut, With<WornBy>>,
    grids: ShoveGrids,
    rngs: ShoveRngs,
    mut fell: MessageWriter<FallOccurred>,
    mut injuries: MessageWriter<InjuryInflicted>,
) {
    // `bevy-traps.md` #1: fail closed (no panic) on the three REQUIRED battle-lifetime resources
    // a shove reads — the surface grid (the support / drop scan), the occupancy grid (the block
    // check), and the tuning (the shove TU + the fall magnitude). All THREE are sim-set at setup;
    // a focused harness that opens BattleInProgress without them resolves no shove (no panic).
    let (Some(surface), Some(occupancy), Some(tuning)) =
        (grids.surface, grids.occupancy, grids.tuning)
    else {
        return;
    };
    // Likewise the two fall streams — a shove with no fall never touches them, but a shove that
    // DOES fall needs both, so gate up front (fail-closed). Both are sim-set at setup.
    let (Some(mut severity_rng), Some(mut injury_rng)) = (rngs.severity, rngs.injury) else {
        return;
    };
    // The injury CONTENT (tables + registry) is Load-OWNED, NOT sim-set (the dispatch_fire
    // Option-with-empty-fallback precedent) — so a sim-only harness (or a battle opened without
    // the Load flow) may have neither. A non-falling shove never touches them; a falling shove's
    // §8 roll then finds no bucket (inflicting no injury) but STILL takes its one draw
    // (content-independent stream alignment), exactly as the ranged fire path. Fall back to
    // empty defaults rather than bailing — else a plain (non-falling) shove would wrongly no-op
    // in a harness that has no injury content (the bug that silently killed the auto-shove).
    let empty_tables = InjuryTables::default();
    let empty_registry = InjuryRegistry::default();
    let tables: &InjuryTables = grids.tables.as_deref().unwrap_or(&empty_tables);
    let registry: &InjuryRegistry = grids.registry.as_deref().unwrap_or(&empty_registry);

    for request in requests.read() {
        // (1) Snapshot the shover + target gate reads (Copy) via `get()` (immutable access on the
        //     mutable query — released before the `get_mut()` fold below, so the read + write of
        //     the SAME query never overlap: the apply_falls single-query precedent). A missing
        //     read component skips (fail-closed). The query tuple is
        //     (Position, Hp, Wounds, LifeState, InflictedWounds, Toughness, Luck, Faction).
        let Ok((&shover_pos, _, _, _, _, _, _, &shover_faction)) = gangers.get(request.shover)
        else {
            continue;
        };
        let Ok((&target_pos, _, _, &target_life, _, _, _, &target_faction)) =
            gangers.get(request.target)
        else {
            continue;
        };

        // (2) Gate + charge — DELIBERATE only. The weapon-tag auto-shove was pre-earned by the
        //     connecting attack (the connect gated + the attack's TU was spent), so it skips.
        if request.source == ShoveSource::Deliberate {
            // The deliberate act: an 8-adjacent, opposing, ALIVE target only (mirrors the melee
            // ganger-arm gates). A rejected gate is a no-op (no TU, no shove).
            if !is_8_adjacent(shover_pos, target_pos)
                || shover_faction == target_faction
                || !target_life.is_active()
            {
                continue;
            }
            // Spend the shove TU off the shover (saturating). The act costs TU whether or not the
            // displacement lands a fall (the melee-swing / fire() TU-charge precedent). A missing
            // Tu pool fails the act (fail-closed).
            let Ok(mut shover_tu) = tu_q.get_mut(request.shover) else {
                continue;
            };
            spend_tu(&mut shover_tu, Tu::new(*tuning.shove_tu));
        }

        // (3) Resolve the one-cell displacement away from the shover (the SHARED verb) — NO RNG.
        let outcome = resolve_shove(shover_pos, target_pos, request.target, &surface, &occupancy);

        // (4) Apply — rewrite the target's Position, and on a fall run the shared GTW-523 fork.
        //     Re-fetch the target's &mut surfaces (the gate snapshot above is released).
        let Ok((position, hp, wounds, life, inflicted, &toughness, &luck, _)) =
            gangers.get_mut(request.target)
        else {
            continue;
        };
        apply_shove(
            outcome,
            ShoveTargetSurfaces {
                position: position.into_inner(),
                hp: hp.into_inner(),
                wounds: wounds.into_inner(),
                life: life.into_inner(),
                inflicted: inflicted.into_inner(),
                toughness,
                luck,
            },
            request.target,
            &wears,
            &mut pieces,
            ShoveFallEnv {
                tuning: &tuning,
                tables,
                registry,
                severity_rng: &mut severity_rng,
                injury_rng: &mut injury_rng,
            },
            &mut fell,
            &mut injuries,
        );
    }
}
