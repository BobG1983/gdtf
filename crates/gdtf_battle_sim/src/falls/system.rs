//! The **falls** system (GTW-523 C1 / C2 / C3 / C5 / C6 / C7) — [`apply_falls`], which
//! reads the buffered [`SlabDestroyed`] signal and, for each destroyed `(cell, level)`,
//! drops every LIVE ganger keyed to that SAME `(cell, level)` to the storey below, applies
//! the weight-free fall damage + injury through the shared pipeline, and emits the
//! [`FallOccurred`] signal.

use bevy::{
    ecs::system::SystemParam,
    prelude::{Entity, MessageReader, MessageWriter, Query, Res, ResMut},
};

use crate::{
    acts::InjuryInflicted,
    armor::{BodyPart, PieceArmorMut, Wears, WornBy},
    effects::on_death::OnDeathOccurred,
    falls::{
        FallOccurred,
        damage::{FallImpact, FallWoundEnv, resolve_fall_hit},
        resolve::resolve_drop,
    },
    ganger::{Hp, LifeState, Luck, Position, Toughness, Wounds},
    inflicted_wound::InflictedWounds,
    injuries::{InjuryRegistry, InjuryTables},
    metric::{CellLevel, Level},
    occupancy::OccupancyGrid,
    occupancy_sync::SlabDestroyed,
    resolve_and_apply::{StruckPiece, TargetGanger},
    rng::{InjuryRng, SeverityRng},
    surface::SurfaceGrid,
    tuning::CombatTuning,
};

/// The mutable **faller surfaces** query — every LIVE ganger's `Position` (the C1 predicate
/// key + the C2 one-shot rewrite) plus the four `&mut` battle surfaces + the read attribute
/// stats + injury ledger the §6/§8 fold needs, factored into a `type` (the
/// [`MeleeTargetQuery`](crate::acts::dispatch_melee) precedent).
///
/// Mutable on `Position` (the involuntary drop write, C2), `Hp` / `Wounds` / `LifeState` /
/// `InflictedWounds` (the [`apply_hit`](crate::apply_hit::apply_hit) fold), read-only on
/// `Toughness` / `Luck` (the §6 severity inputs, read off the already-injury-projected
/// derived-stat components — the `fold_ganger` precedent that uses the RAW `TargetGanger`
/// Toughness / Luck). One query drives BOTH the predicate scan (iterate all rows, match
/// `(cell, level)`) and the fold (mutate the matched row in place) — no second disjoint query
/// is needed because the predicate reads and the fold writes are on the SAME entity row.
type FallerQuery<'world, 'state> = Query<
    'world,
    'state,
    (
        Entity,
        &'static mut Position,
        &'static mut Hp,
        &'static mut Wounds,
        &'static mut LifeState,
        &'static mut InflictedWounds,
        &'static Toughness,
        &'static Luck,
    ),
>;

/// The world grids + tuning + injury content the falls fold reads, bundled into one
/// [`SystemParam`] so [`apply_falls`] stays under Bevy's 16-param system limit (the
/// [`MeleeGrids`](crate::acts::dispatch_melee) grouping precedent).
///
/// Every field is a battle-lifetime resource taken `Option<Res<T>>` so a focused harness
/// that opens `BattleInProgress` WITHOUT the full setup flow (a `bleed` / `battle-outcome` /
/// `terrain-entity` unit test) does not panic this runtime system on a resource's absence
/// (`bevy-traps.md` #1; the sibling [`FallRngs`] `Option<ResMut>` precedent): the persistent
/// [`SurfaceGrid`] (the C2 drop scan's support column), the [`OccupancyGrid`] (the C3
/// stair-brace read), the [`CombatTuning`] (the C4 per-storey magnitude + the §6 severity
/// scaling + wound costs), and the [`InjuryTables`] / [`InjuryRegistry`] (the C5 injury
/// roll's shared pool). With any absent, [`apply_falls`] resolves no fall (a safe, defined
/// fallback — never a panic); in the real app all are sim-set at `setup_battle`, so a live
/// fall always has them. A transparent system-param bundle of named world-state resources.
#[derive(SystemParam)]
pub struct FallGrids<'w> {
    /// The persistent surface grid — the C2 drop scan reads its slab column downward for
    /// the highest supported storey.
    surface:   Option<Res<'w, SurfaceGrid>>,
    /// The coarse occupancy grid — the C3 stair-brace read (a stair lower-endpoint occupant
    /// is braced and does not fall through its own stair).
    occupancy: Option<Res<'w, OccupancyGrid>>,
    /// The combat tuning — the C4 `per_storey_damage` leaf + the §6 severity scaling / wound
    /// costs the fall's damage + severity fold reads.
    tuning:    Option<Res<'w, CombatTuning>>,
    /// The shared weighted `(category, severity)` injury tables — the C5 injury roll's pool
    /// (used AS-IS; NO new source dimension — GTW-452 owns falling weighting).
    tables:    Option<Res<'w, InjuryTables>>,
    /// The injury registry — resolves the C5 roll's picked key to its authored def.
    registry:  Option<Res<'w, InjuryRegistry>>,
}

/// The two seeded draw streams the §6 / §8 fall-damage fold advances, taken
/// `Option<ResMut<…>>` so a focused harness that opens `BattleInProgress` WITHOUT the full
/// setup flow does not panic this runtime system on a stream's absence (`bevy-traps.md` #1;
/// the [`MeleeRngs`](crate::acts::dispatch_melee) `Option<ResMut>` precedent).
///
/// REUSES [`SeverityRng`] + [`InjuryRng`] verbatim — **no new RNG stream, no `FightRng`
/// draw** (a slab-destroy fall has NO attacker, so no opposed roll — GTW-523 C7). With
/// either absent, [`apply_falls`] resolves no fall damage (a safe, defined fallback — never
/// a panic). In the real app both are sim-set (inserted at `setup_battle`), so a live fall
/// always has them.
#[derive(SystemParam)]
pub struct FallRngs<'w> {
    /// The §6 severity-roll stream — one draw per faller.
    severity: Option<ResMut<'w, SeverityRng>>,
    /// The §8 injury-roll stream — one draw per faller with a non-graze / non-fatal wound.
    injury:   Option<ResMut<'w, InjuryRng>>,
}

/// The armor-relationship queries the struck-piece resolution reads (`faller → Wears → the
/// BodyPart-tagged piece`, GTW-323 / ADR-0004), bundled so [`apply_falls`] stays under the
/// param limit. Disjoint from [`FallerQuery`] (a different component / entity set).
#[derive(SystemParam)]
pub struct FallArmor<'w, 's> {
    /// The `faller → Wears → piece entities` relationship read.
    wears:  Query<'w, 's, &'static Wears>,
    /// The worn-armor-piece entities' stats — the struck piece's `&mut ArmorIntegrity` (the
    /// wear path) + its read stats.
    pieces: Query<'w, 's, PieceArmorMut, bevy::prelude::With<WornBy>>,
}

/// The three buffered output signals [`apply_falls`] emits per fall, bundled into one
/// [`SystemParam`] so the system stays under clippy's argument-count gate (the sibling
/// [`FallGrids`] / [`FallRngs`] / [`FallArmor`] grouping precedent).
///
/// A fall emits: the presenter-facing [`FallOccurred`] (once per fall, C6); the EXISTING
/// [`InjuryInflicted`] bridge (a rolled injury, C5); and — because a fall CAN kill — the
/// GTW-547 [`OnDeathOccurred`] terminal-death signal (a fall-killed faller, at its landing
/// cell) so `resolve_on_death` fans its authored on-death effect. A transparent bundle of the
/// three independent `MessageWriter`s (framework plumbing — no bare domain type).
#[derive(SystemParam)]
pub struct FallSignals<'w> {
    /// The presenter-facing fall signal — one per resolved fall (C6).
    fell:     MessageWriter<'w, FallOccurred>,
    /// The EXISTING injury bridge — a rolled fall injury addressed to the faller (C5).
    injuries: MessageWriter<'w, InjuryInflicted>,
    /// The GTW-547 terminal-death signal — a fall-KILLED faller, at its landing cell, so
    /// `resolve_on_death` fans its authored on-death effect (the falls terminal-death gate).
    deaths:   MessageWriter<'w, OnDeathOccurred>,
}

/// **Apply falls** for every buffered [`SlabDestroyed`] — the GTW-523 authoritative fall
/// mechanic for trigger (a): a slab destroyed under a standing ganger.
///
/// For each buffered [`SlabDestroyed`] `{ at: (cell, level) }`:
///
/// 1. **Faller predicate (C1).** Scan every LIVE ([`LifeState::is_active`]) ganger; a faller
///    is one whose [`Position`] is at that SAME `(cell, level)` — `Position.level == level`,
///    the SAME storey the destroyed slab floors (NOT `level + 1`: the slab keyed `(cell, N)`
///    is the FLOOR of storey `N` per the march boundary convention, so the ganger STANDING
///    on it has `Position.level == N`; `N + 1` is the ROOF — the wrong actor). ALL matching
///    gangers fall (C7 multi-faller).
/// 2. **Stair brace (C3).** A faller whose cell is an authored stair tile
///    ([`OccupancyGrid::is_stair_cell`]) is BRACED and does NOT fall through its own stair —
///    skipped (no drop, no damage, no signal).
/// 3. **Drop resolution (C2).** [`resolve_drop`] scans the cell's slab column downward for
///    the highest supported storey (`k == 0` ground, or a `Present` slab; `Absent` = open
///    air, `Destroyed` = a hole). The faller's [`Position`] is rewritten to `(cell, landing)`
///    as ONE involuntary write — `Changed<Position>` then drives
///    [`sync_moved_gangers`](crate::occupancy_sync::sync_moved_gangers) + `PrevSlot` teardown
///    automatically (the occupancy grid is NEVER hand-edited here).
/// 4. **Fall damage + injury (C4 / C5).** [`resolve_fall_hit`] routes
///    `per_storey_damage × storeys` as a [`Matchup::Neutral`](crate::matchup::Matchup) kinetic
///    hit through the SAME `resolve_hit` → `roll_severity` (one [`SeverityRng`] draw) →
///    `apply_hit` → `roll_injury` (one [`InjuryRng`] draw on a non-graze / non-fatal wound)
///    pipeline `fold_ganger` runs — armor honored, only weight deferred. The struck part is
///    [`BodyPart::Torso`] (the body lands as a whole — deterministic, NO body-part draw, so
///    the draw discipline is exactly one severity + one injury per faller).
/// 5. **Emit (C5 / C6 / GTW-547).** A rolled injury is bridged to the EXISTING
///    [`InjuryInflicted`] message (the `dispatch_fire` bridge precedent); a [`FallOccurred`]
///    is emitted per fall; and — because a fall CAN kill (the shared wound core flips
///    [`LifeState::Dead`] on `Wounds -> 0`) — a fall-killed faller ALSO emits an
///    [`OnDeathOccurred`] at its landing cell, so `resolve_on_death` fans its authored
///    on-death effect (the falls terminal-death gate, GTW-547).
///
/// # Determinism / ordering (C7)
///
/// REUSES [`SeverityRng`] + [`InjuryRng`] (no new stream, no `FightRng` — a fall has no
/// attacker). Multi-faller iteration is over the query's stable order for a fixed
/// destroyed-cell order, so the same [`BattleSeed`](crate::rng::BattleSeed) + same event order
/// yields identical outcomes. Ordered EXPLICITLY `.after(dispatch_fire)` (so the SAME-FRAME
/// `SlabDestroyed` is buffered when this reads it) and `.after(sync_destroyed_slab)` (so the
/// slab is already `Destroyed` on the surface grid — the two share the same-frame
/// `SlabDestroyed` buffer via INDEPENDENT `MessageReader` cursors, never stealing) — see
/// [`FallsPlugin`](super::FallsPlugin).
///
/// Param-only (`bevy-traps.md` #7 — no `&mut World`). Fail-closed: a missing stream, a
/// braced faller, or a `start == 0` ganger simply does not fall (never a panic). The
/// per-concern grids / tuning / injury content ([`FallGrids`]) + the two draw streams
/// ([`FallRngs`]) + the armor relationship queries ([`FallArmor`]) + the three output signals
/// ([`FallSignals`]) are grouped into [`SystemParam`] bundles, keeping the system's own param
/// count under clippy's gate.
pub fn apply_falls(
    mut destroyed: MessageReader<SlabDestroyed>,
    mut fallers: FallerQuery,
    mut armor: FallArmor,
    grids: FallGrids,
    rngs: FallRngs,
    mut signals: FallSignals,
) {
    // `bevy-traps.md` #1: without both seeded streams no fall can resolve a draw — fail closed
    // (no panic) rather than reading an absent battle-lifetime resource. In the real app both
    // are sim-set (inserted at setup), so this never bails there (the dispatch_melee /
    // reaction_trigger `Option<ResMut>` precedent).
    let (Some(mut severity_rng), Some(mut injury_rng)) = (rngs.severity, rngs.injury) else {
        return;
    };

    // `bevy-traps.md` #1: likewise fail-closed on the battle-lifetime world grids / tuning /
    // injury content — a focused harness that opens `BattleInProgress` without the full setup
    // flow may not have seeded them, so a required `Res` would panic param-validation. In the
    // real app all are sim-set at setup, so this never bails there.
    let (Some(surface), Some(occupancy), Some(tuning), Some(tables), Some(registry)) = (
        grids.surface,
        grids.occupancy,
        grids.tuning,
        grids.tables,
        grids.registry,
    ) else {
        return;
    };

    for event in destroyed.read() {
        // The canonical CellLevel::split decompose (GTW-565).
        let (destroyed_cell, destroyed_level) = event.at.split();

        // C1: iterate ALL gangers; a faller is any LIVE ganger keyed to the SAME (cell, level)
        // as the destroyed slab (level == the slab's floor storey, NOT level + 1). C7: every
        // matching ganger falls (multi-faller), in the query's stable iteration order.
        for (entity, mut position, mut hp, mut wounds, mut life, mut inflicted, toughness, luck) in
            &mut fallers
        {
            // Only a LIVE (Alive) ganger falls — a corpse/downed body is not a standing actor
            // for trigger (a). (A Downed body HOLDS its cell but is not "standing"; the fall
            // mechanic is for a standing ganger — resolution.md §Falls.)
            if !life.is_active() {
                continue;
            }
            // The C1 key match: SAME cell AND SAME level as the destroyed slab (the
            // canonical CellLevel accessors through Position's deref, GTW-565).
            if position.cell() != destroyed_cell || position.level() != destroyed_level {
                continue;
            }

            // C3: a stair lower-endpoint occupant is BRACED — it does not fall through its own
            // stair (the stair supports it). Skip (no drop, no damage, no signal).
            if occupancy.is_stair_cell(&position) {
                continue;
            }

            // C2: resolve the drop — scan down for the highest supported storey. A start on the
            // ground (level 0) returns None (nothing to fall to) — fail-closed, no drop.
            let start = position.level();
            let Some(landing) = resolve_drop(destroyed_cell, start, &surface) else {
                continue;
            };

            // C2: rewrite Position to (cell, landing) as ONE involuntary write. Changed<Position>
            // drives sync_moved_gangers + PrevSlot teardown automatically next frame (the
            // dispatch_move precedent) — the occupancy grid is NEVER hand-edited here.
            *position = Position::new(CellLevel::new(destroyed_cell, landing.landing));

            // C4/C5: the struck piece resolution — `faller → Wears → the Torso-tagged piece`
            // (the strike_with_target precedent). A faller wearing nothing folds to bare flesh.
            // The fall lands on the whole body → BodyPart::Torso (deterministic, NO body-part
            // draw — the draw discipline is exactly one severity + one injury per faller).
            let part = BodyPart::Torso;
            let piece_view = armor
                .wears
                .get(entity)
                .ok()
                .and_then(|worn| worn.pieces().next())
                .and_then(|piece_entity| {
                    armor
                        .pieces
                        .get_mut(piece_entity)
                        .ok()
                        .map(|piece| StruckPiece {
                            floor:      *piece.floor,
                            protection: *piece.protection,
                            hardness:   *piece.hardness,
                            armor_type: *piece.armor_type,
                            integrity:  piece.integrity.into_inner(),
                        })
                });

            // C4/C5: the weight-free damage + injury synthesis through the SHARED
            // wound-synthesis core (corpse-skip → resolve_hit → roll_severity[1 SeverityRng]
            // → apply_hit → roll_injury[1 InjuryRng on non-graze/non-fatal]) the ganger path
            // also routes through. REUSES the resolve/apply + injury verbs verbatim — the fall
            // inputs are grouped into the FallImpact bundle, the shared content + streams into
            // the FallWoundEnv bundle (so the fork's signature needs no argument-count suppression).
            let rolled = resolve_fall_hit(
                FallImpact {
                    per_storey: tuning.per_storey_damage,
                    storeys: landing.storeys,
                    part,
                    target: TargetGanger {
                        hp:        &mut hp,
                        wounds:    &mut wounds,
                        life:      &mut life,
                        piece:     piece_view,
                        inflicted: &mut inflicted,
                        toughness: *toughness,
                        luck:      *luck,
                    },
                    target_entity: entity,
                },
                FallWoundEnv {
                    tuning:       &tuning,
                    tables:       &tables,
                    registry:     &registry,
                    severity_rng: &mut severity_rng,
                    injury_rng:   &mut injury_rng,
                },
            );

            // C5 / C6 / GTW-547: emit the three per-fall output signals (the rolled injury
            // bridge, the fall-killed OnDeathOccurred, the FallOccurred) off the post-fold
            // verdict, factored out so the loop body stays under clippy's line gate.
            emit_fall_signals(
                &mut signals,
                entity,
                *life,
                &position,
                start,
                landing,
                rolled,
            );
        }
    }
}

/// Emit the three output signals one resolved fall produces — the rolled injury bridge (C5),
/// the fall-killed [`OnDeathOccurred`] (GTW-547), and the [`FallOccurred`] (C6) — off the
/// post-fold faller state. Factored out of the [`apply_falls`] loop body so it stays under
/// clippy's per-function line gate; every write mirrors its inlined precedent verbatim.
///
/// - **C5.** A rolled injury bridges to the EXISTING [`InjuryInflicted`] message (the
///   `dispatch_fire` bridge precedent), addressed to the faller entity.
/// - **GTW-547.** A FALL can KILL (the shared wound core flips [`LifeState::Dead`] on
///   `Wounds -> 0`), so falls is a terminal death gate like the fire / melee / bleed / DOT /
///   field kills: a fall-killed faller emits [`OnDeathOccurred`] at its LANDING cell (the same
///   `(cell, level)` the fire / melee ganger-kill bridge uses) so `resolve_on_death` fans its
///   authored on-death effect. Read off the frozen post-fold `life` (the `emit_on_death`
///   precedent) — a fall that wounded-but-did-not-kill emits nothing here.
/// - **C6.** A [`FallOccurred`] is emitted once per fall.
fn emit_fall_signals(
    signals: &mut FallSignals,
    entity: Entity,
    life: LifeState,
    position: &Position,
    start: Level,
    landing: crate::falls::DropLanding,
    rolled: Option<crate::injuries::RolledInjury>,
) {
    if let Some(rolled) = rolled {
        signals
            .injuries
            .write(InjuryInflicted::from_rolled(entity, rolled));
    }
    if life == LifeState::Dead {
        signals
            .deaths
            .write(OnDeathOccurred::new(entity, **position));
    }
    signals.fell.write(FallOccurred::new(
        entity,
        start,
        landing.landing,
        landing.storeys,
    ));
}
