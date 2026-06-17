//! The **fire** dispatch — the GTW-242 firing-arc + turn-to-fire gate over a buffered
//! [`FireRequested`], then the landed [`fire`] verb (E10.2 AC3 / GTW-242).
//!
//! No act logic is reimplemented here: the shot resolution REUSES [`fire`] verbatim; this
//! slice only GATES it (the firing arc + the affordable turn-into-arc) and front-loads the
//! turn. The two queries that both touch `Facing`/`Tu` are time-multiplexed through a
//! [`ParamSet`] (`bevy-traps.md` #3 / #7 — no `&mut World`).

use bevy::{
    ecs::system::{ParamSet, SystemParam},
    prelude::{MessageReader, Query, Res, ResMut},
};

use crate::{
    acts::request::FireRequested,
    cover::CoverLedger,
    fire::{BattleGrids, FireOrder, ShooterQuery, TargetQuery, fire},
    firing_arc::target_in_arc,
    ganger::{Aiming, Direction, Facing, Position, Tu, TuMax},
    magazine::mode_tu_cost,
    metric::Cell,
    occupancy::OccupancyGrid,
    rng::SimRng,
    surface::SurfaceGrid,
    tu::spend_tu,
    tuning::CombatTuning,
};

/// The three change-driven world-grid resources [`dispatch_fire`] reads, bundled into one
/// [`SystemParam`] so the system's parameter list stays under clippy's argument-count gate
/// (the [`BattleGrids`] / [`FireOrder`] grouping precedent in `fire.rs`).
///
/// Grouping the cohesive grid `Res<…>` reads into one param keeps [`dispatch_fire`] at
/// seven parameters; the body assembles the borrow-based [`BattleGrids`] from these `Res`
/// reads via [`BattleGridsParam::grids`]. A transparent system-param bundle of existing
/// named world-state resources — not itself a wrapped domain scalar.
#[derive(SystemParam)]
pub struct BattleGridsParam<'w> {
    /// The coarse 3D occupancy grid — the march's collision / occupant-band surface.
    occupancy: Res<'w, OccupancyGrid>,
    /// The persistent floor/roof-slab + ground surface grid the march flies through.
    surface:   Res<'w, SurfaceGrid>,
    /// The model cover ledger — peeked for the faced cell + the target cell's cover band.
    cover:     Res<'w, CoverLedger>,
}

impl BattleGridsParam<'_> {
    /// Assemble the borrow-based [`BattleGrids`] [`fire`] reads from these grid `Res`
    /// reads (`Res<T>` derefs to `&T`) — the grids are read, never rebuilt.
    fn grids(&self) -> BattleGrids<'_> {
        BattleGrids {
            occupancy: &self.occupancy,
            surface:   &self.surface,
            cover:     &self.cover,
        }
    }
}

/// The query the GTW-242 fire dispatch turns the shooter through for an out-of-arc shot —
/// the actor's `(&mut `[`Facing`]`, &mut `[`Tu`]`)`, the two components a turn-to-fire
/// mutates (set the new facing + debit the turn TU).
///
/// It conflicts with [`ShooterQuery`] (which reads `&Facing` and writes `&mut Tu`), so the
/// two CANNOT be independent system params — they are time-multiplexed through a
/// [`ParamSet`] (`bevy-traps.md` #3: a deliberate, ordered access of the same components at
/// distinct points in the system, never an ambiguous overlap). The turn write happens
/// FIRST (front-loaded), the [`ShooterQuery`] re-borrow for [`fire`] SECOND.
type TurnQuery<'world, 'state> = Query<'world, 'state, (&'static mut Facing, &'static mut Tu)>;

/// The firing-arc gate's decision for ONE [`FireRequested`] — computed from the read state
/// BEFORE any mutation, so the spend/turn/shot is atomic-by-construction (GTW-242).
///
/// The arc rule (`docs/combat/resolution.md` §1 / the targeting section, USER ruling
/// 2026-06-16): an in-arc target fires directly; an out-of-arc target fires ONLY when the
/// shooter affords BOTH the turn-into-arc AND the shot — else the shot is REJECTED (no TU
/// spent, no facing change, no shot). This enum carries the pre-computed verdict so the
/// dispatch performs exactly the mutations the verdict allows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FireArcDecision {
    /// In-arc — fire directly (no turn). [`fire`]'s own [`crate::magazine::can_fire`] gate
    /// handles fire-TU affordability (an in-arc shot the shooter cannot afford resolves to
    /// nothing inside [`fire`], mutating nothing).
    FireInArc,
    /// Out-of-arc AND affordable — turn to `facing` (spending `turn_cost`) THEN fire.
    TurnThenFire {
        /// The compass facing toward the target ([`Direction::from_cells`]).
        facing:    Direction,
        /// The full turn-into-arc TU cost (`steps_to × turn_tu`), spent before the shot.
        turn_cost: Tu,
    },
    /// Out-of-arc and NOT affordable (cannot pay turn + fire) — REJECT: no spend, no turn,
    /// no shot.
    Reject,
}

/// Decide the firing-arc verdict for a shot from `actor_cell` (facing `facing`, current
/// pool `tu`) at `target_cell`, given the fire-TU cost and the [`CombatTuning`] (firing arc
/// + per-45°-step turn TU) — the pure GTW-242 gate, computed before any world mutation.
///
/// In-arc ([`target_in_arc`]) ⇒ [`FireArcDecision::FireInArc`]. Out-of-arc ⇒ compute the
/// target facing ([`Direction::from_cells`] — a co-located target is never out-of-arc, so
/// this is `Some`; a defensive `None` falls back to a direct shot) and the full turn cost
/// (`steps_to × turn_tu`, REUSING the landed [`Direction::steps_to`] + the
/// [`crate::tuning::TurnTu`] leaf — NOT re-derived); affordable iff `tu ≥ turn_cost +
/// fire_cost` ([`FireArcDecision::TurnThenFire`]), else [`FireArcDecision::Reject`]. The
/// `fire_cost`/`turn_cost` arithmetic is **saturating** on the `u8` pool (no overflow, no
/// panic). Pure, total.
fn decide_fire_arc(
    facing: Direction,
    actor_cell: Cell,
    target_cell: Cell,
    tu: Tu,
    fire_cost: Tu,
    tuning: &CombatTuning,
) -> FireArcDecision {
    if target_in_arc(facing, actor_cell, target_cell, &tuning.firing_arc) {
        return FireArcDecision::FireInArc;
    }
    // Out-of-arc: the shot needs a turn-into-arc first. from_cells is Some here (a
    // co-located target is in-arc, handled above); a defensive None degrades to a direct
    // shot rather than panicking.
    let Some(target_facing) = Direction::from_cells(actor_cell, target_cell) else {
        return FireArcDecision::FireInArc;
    };
    // The FULL turn cost, REUSING the landed steps_to (short-way 45° count) × the TurnTu
    // leaf — NOT the partial set_facing path. steps_to ∈ 0..=4 and turn_tu is small, so
    // saturating_mul never wraps (and stays panic-free regardless).
    let turn_cost = Tu::new(
        facing
            .steps_to(target_facing)
            .saturating_mul(*tuning.turn_tu),
    );
    // The COMBINED gate (the user's stricter ruling): afford BOTH turn AND fire, or reject.
    // Saturating add keeps the sum panic-free even if the costs were pathologically large.
    let combined = (*turn_cost).saturating_add(*fire_cost);
    if *tu >= combined {
        FireArcDecision::TurnThenFire {
            facing: target_facing,
            turn_cost,
        }
    } else {
        FireArcDecision::Reject
    }
}

/// **Dispatch** buffered [`FireRequested`] messages with the GTW-242 **firing-arc +
/// turn-to-fire** gate, then run the landed [`fire`] verb (E10.2 AC3 / GTW-242).
///
/// For each request the gate is computed from the read state BEFORE any mutation
/// ([`decide_fire_arc`]), so the spend/turn/shot is atomic-by-construction
/// (`docs/combat/resolution.md` §1; USER ruling 2026-06-16):
///
/// - **In-arc** ([`FireArcDecision::FireInArc`]): run [`fire`] directly (facing unchanged).
///   [`fire`]'s own [`crate::magazine::can_fire`] gate + [`crate::tuning::TurnTu`]-free
///   charge handle fire-TU affordability — an unaffordable in-arc shot resolves to nothing.
/// - **Out-of-arc + affordable** ([`FireArcDecision::TurnThenFire`]): ATOMICALLY spend the
///   turn TU + set the new [`Facing`] (the [`TurnQuery`] half of the [`ParamSet`]) THEN run
///   [`fire`] (the [`ShooterQuery`] half), which spends the fire TU and resolves the shot.
///   The combined gate guarantees the remaining pool still affords [`fire`]'s charge.
/// - **Out-of-arc + unaffordable** ([`FireArcDecision::Reject`]): no TU spent, no facing
///   change, no shot — `continue`.
///
/// The turn-write query ([`TurnQuery`]) and [`ShooterQuery`] both touch `Facing`/`Tu`, so
/// they are time-multiplexed through a [`ParamSet`] (`bevy-traps.md` #3 / #7 — no
/// `&mut World`); the turn write is taken FIRST, the [`fire`] re-borrow SECOND. No act
/// logic is reimplemented — the shot resolution REUSES [`fire`] verbatim; this slice only
/// GATES it and front-loads the turn.
pub fn dispatch_fire(
    mut requests: MessageReader<FireRequested>,
    mut shooter_set: ParamSet<(ShooterQuery, TurnQuery)>,
    mut targets: TargetQuery,
    grids: BattleGridsParam,
    tuning: Res<CombatTuning>,
    mut rng: ResMut<SimRng>,
) {
    for request in requests.read() {
        // (1) READ the arc-relevant shooter state through the ShooterQuery half, copying
        //     every Copy value out so the query borrow ends at the block boundary (freeing
        //     the ParamSet to lend p1 below). A shooter not in the query (unarmed /
        //     despawned) fires nothing (fail-closed).
        let shooters = shooter_set.p0();
        let Ok(((position, facing, _, aiming, _, _, tu_max), (tu, _), _)) =
            shooters.get(request.shooter)
        else {
            continue;
        };
        let actor_cell = actor_cell(position);
        let facing: Direction = **facing;
        let tu: Tu = *tu;
        let tu_max: TuMax = *tu_max;
        let aiming: Aiming = *aiming;

        // (2) The fire-TU cost — the EXISTING fire-act charge (mode_tu_cost), the same
        //     source fire()'s own debit reads; both gates agree on the cost.
        let fire_cost = mode_tu_cost(&request.mode, &tu_max, &aiming, &tuning);

        // (3) The arc verdict, computed BEFORE any mutation (atomic-by-construction).
        match decide_fire_arc(
            facing,
            actor_cell,
            request.target_cell,
            tu,
            fire_cost,
            &tuning,
        ) {
            FireArcDecision::Reject => continue, // unaffordable turn+fire: no spend, no shot
            FireArcDecision::TurnThenFire {
                facing: target_facing,
                turn_cost,
            } => {
                // ATOMICALLY front-load the turn through the TurnQuery half: set the new
                // facing + spend the turn TU. The combined gate guarantees the remaining
                // pool still affords fire()'s charge below.
                let mut turners = shooter_set.p1();
                let Ok((mut actor_facing, mut actor_tu)) = turners.get_mut(request.shooter) else {
                    continue;
                };
                *actor_facing = Facing::new(target_facing);
                spend_tu(&mut actor_tu, turn_cost);
                // `turners` (the p1 borrow) ends with this match arm's block, freeing the
                // ParamSet to re-lend p0 for fire() below — no explicit drop needed.
            }
            FireArcDecision::FireInArc => {} // direct shot: no turn, fall through to fire()
        }

        // (4) Run the landed verb ONCE (REUSED verbatim) — it spends the fire TU and
        //     resolves the shot. The dropped volley's effects are the in-world mutations
        //     (TU / ammo / target surfaces) the presenter observes via change-detection.
        let order = FireOrder {
            mode:         &request.mode,
            target_cell:  request.target_cell,
            target_level: request.target_level,
        };
        let mut shooters = shooter_set.p0();
        let _volley = fire(
            request.shooter,
            order,
            &mut shooters,
            &mut targets,
            grids.grids(),
            &tuning,
            &mut rng,
        );
    }
}

/// The ground-plane [`Cell`] of a shooter's [`Position`] — its `(x, y)` (the `z` storey is
/// irrelevant to a ground facing). [`Position`] derefs to [`CellLevel`](crate::metric::CellLevel),
/// which derefs to the inner `IVec3`; the cell is its `x`/`y` (the [`crate::faced_cell`]
/// split precedent).
fn actor_cell(position: &Position) -> Cell {
    let key = ***position;
    Cell::new(key.x, key.y)
}
