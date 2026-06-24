//! The **fire** dispatch — the GTW-242 firing-arc + turn-to-fire gate over a buffered
//! [`FireRequested`], then the landed [`fire`] verb (E10.2 AC3 / GTW-242).
//!
//! No act logic is reimplemented here: the shot resolution REUSES [`fire`] verbatim; this
//! slice only GATES it (the firing arc + the affordable turn-into-arc) and front-loads the
//! turn. The two queries that both touch `Facing`/`Tu` are time-multiplexed through a
//! [`ParamSet`] (`bevy-traps.md` #3 / #7 — no `&mut World`).

use bevy::{
    ecs::system::{ParamSet, SystemParam},
    prelude::{Entity, Message, MessageReader, MessageWriter, Query, Res, ResMut},
};

use crate::{
    acts::request::FireRequested,
    cover::CoverLedger,
    fire::{
        BattleGrids, FireOrder, PieceQuery, ShooterQuery, TargetQuery, WeaponQuery, WearsQuery,
        WieldsQuery, fire,
    },
    firing_arc::target_in_arc,
    ganger::{Aiming, Direction, Facing, Position, Tu, TuMax},
    magazine::mode_tu_cost,
    metric::{Cell, CellLevel},
    occupancy::OccupancyGrid,
    occupancy_sync::CoverDestroyed,
    rng::SimRng,
    shot_fired::ShotFired,
    surface::SurfaceGrid,
    tu::spend_tu,
    tuning::CombatTuning,
    weapon::{DamageType, ModeKind, Wields},
};

/// A **fire was declared** — the combat-log signal that `shooter` fired `mode` at
/// `target` (GTW-328), emitted ONCE per [`FireRequested`] that passes the firing-arc gate,
/// BEFORE the shot rolls.
///
/// The combat-text LOG event for a shot declaration ("<name> fired <Single/Burst/Full> at
/// <target>") — the user-facing announcement that a shot is being taken, distinct from the
/// per-round [`ShotFired`] outcome signal (a burst declares ONCE but fires multiple
/// rounds). It carries ONLY data the [`dispatch_fire`] system already holds at fire time —
/// the [`shooter`](FireDeclaration::shooter) ref, the resolved [`target`](FireDeclaration::target)
/// occupant entity (if the aimed cell holds one, else `None`), and the
/// [`mode`](FireDeclaration::mode) [`ModeKind`] (read off the request's
/// [`FireModeSpec`](crate::weapon::FireModeSpec) kind). It adds **no** fire-result logic,
/// performs **no** RNG draw, and re-resolves nothing — the determinism property is
/// untouched (`docs/combat/resolution.md` §"What's pure math vs sim").
///
/// A buffered Bevy [`Message`] (`bevy-traps.md` #4 — NOT the observer `Event`), written
/// with [`MessageWriter`] and read with [`MessageReader`], mirroring [`ShotFired`] /
/// [`ReloadResult`](crate::acts::ReloadResult). The [`shooter`](FireDeclaration::shooter) /
/// [`target`](FireDeclaration::target) are Bevy [`Entity`] handles — framework plumbing,
/// the only bare type the no-bare-types rule permits in a payload; [`mode`](FireDeclaration::mode)
/// is the domain [`ModeKind`] enum, never a bare label string.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FireDeclaration {
    /// The firing entity (the armed shooter the round leaves) — resolved to a name by the
    /// combat-log presenter via `Query<&GangerName>`.
    pub shooter: Entity,
    /// The intended target occupant entity — the ganger standing in the aimed `(cell,
    /// level)` if one is there (the occupancy grid read the dispatch already holds), else
    /// `None` (the shot is aimed at an empty cell / impact point). NOT a fresh raycast or
    /// re-resolve — a single O(1) grid peek of the data the dispatch already reads.
    pub target:  Option<Entity>,
    /// The declared fire mode's closed kind (`Single` / `Burst` / `Full`) — read off the
    /// request's [`FireModeSpec`](crate::weapon::FireModeSpec) kind; the log renders its
    /// [`Display`](std::fmt::Display) label.
    pub mode:    ModeKind,
}

impl FireDeclaration {
    /// Build a fire-declaration signal for `shooter` firing `mode` at `target` (the
    /// resolved occupant entity, or `None` for an empty-cell shot).
    #[must_use]
    pub const fn new(shooter: Entity, target: Option<Entity>, mode: ModeKind) -> Self {
        Self {
            shooter,
            target,
            mode,
        }
    }
}

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
    /// The model cover ledger — peeked (read) for the faced cell + the target cell's
    /// cover band, and **spent** (write) when a round strikes cover (GTW-364), so it is
    /// a [`ResMut`] now (the cover-hit depletion path writes the ledger's HP in place).
    cover:     ResMut<'w, CoverLedger>,
}

impl BattleGridsParam<'_> {
    /// Assemble the borrow-based [`BattleGrids`] [`fire`] reads (and, GTW-364, the
    /// cover-hit path WRITES) from these grid resources — the occupancy / surface are
    /// read (`Res<T>` derefs to `&T`), the cover is taken `&mut` (the depletion path).
    /// Takes `&mut self` for the mutable cover borrow; the grids are still never
    /// rebuilt — only the struck cover's HP is spent in place.
    fn grids(&mut self) -> BattleGrids<'_> {
        BattleGrids {
            occupancy: &self.occupancy,
            surface:   &self.surface,
            cover:     &mut self.cover,
        }
    }

    /// The occupant entity (if any) at `at` — a single O(1) occupancy peek used for the
    /// GTW-328 fire declaration's resolved target. Borrows only the occupancy grid, so
    /// it does not conflict with the `&mut cover` borrow `grids` hands out.
    fn occupant_at(&self, at: CellLevel) -> Option<Entity> {
        self.occupancy.occupant(&at)
    }
}

/// The output signal [`MessageWriter`]s [`dispatch_fire`] emits on, bundled into one
/// [`SystemParam`] so the system's parameter list stays under clippy's argument-count gate
/// (the [`BattleGridsParam`] grouping precedent above).
///
/// Grouping the cohesive output writers into one param keeps [`dispatch_fire`] under the
/// argument-count gate: the per-round [`ShotFired`] geometry/FCT signal (GTW-290 / GTW-302),
/// the per-request [`FireDeclaration`] combat-log signal (GTW-328), and the per-round
/// [`CoverDestroyed`] signal (GTW-364) — the fire→deplete→message bridge a cover-destroying
/// round emits, which the maintenance + visibility systems consume to free the smashed cell.
/// A transparent system-param bundle of named output buffers — not itself a wrapped domain
/// value.
#[derive(SystemParam)]
pub struct FireSignals<'w> {
    /// The per-ROUND fire-trajectory signal (one per round resolved) — the presenter's
    /// muzzle / tracer / impact FX + the floating-combat-text verdict.
    shots:           MessageWriter<'w, ShotFired>,
    /// The per-REQUEST combat-log declaration (one per proceeding shot) — "<name> fired
    /// <mode> at <target>".
    declarations:    MessageWriter<'w, FireDeclaration>,
    /// The per-ROUND cover-destroyed signal (GTW-364) — emitted for each round whose
    /// [`HitReport::cover_destroyed`](crate::resolve_and_apply::HitReport::cover_destroyed)
    /// is `Some`, bridging the ledger's `deplete_cover` destruction into the buffered
    /// [`CoverDestroyed`] message that `sync_destroyed_cover` + `should_recompute_visibility`
    /// consume to free the cell + reopen LOS.
    cover_destroyed: MessageWriter<'w, CoverDestroyed>,
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
///
/// **The GTW-290 / GTW-302 fire signal.** AFTER the volley resolves (before it is dropped)
/// this emits ONE [`ShotFired`] message per ROUND fired, zipping the parallel
/// [`Volley::shots`](crate::fire::Volley::shots) geometry with the
/// [`Volley::reports`](crate::fire::Volley::reports) verdicts so each message carries BOTH
/// that round's already-computed [`ShotOutcome`](crate::resolve_coarse::ShotOutcome) AND its
/// [`HitReport`](crate::resolve_and_apply::HitReport) ([`ShotFired::from_round`]) — so the
/// presenter draws a muzzle / tracer / impact FX per round (a burst → multiple tracers) AND
/// the floating-combat-text presenter (GTW-302) draws that round's damage / wound / severity
/// / armor verdict. The two vectors are parallel (`reports[i]`/`shots[i]` are the same fired
/// round), so the report rides `Some` for every fired round. This changes NO fire-result
/// logic — it only EXPOSES the trajectory + report the volley already computed (the
/// [`MessageWriter`], `bevy-traps.md` #4 / #7).
#[expect(
    clippy::too_many_arguments,
    reason = "the GTW-323 armor + weapon relationships add the disjoint wears/pieces + \
              wields/weapons system params to the dispatch_fire signature; each is a \
              distinct, independently-borrowed Bevy SystemParam that cannot be bundled \
              without a custom SystemParam struct that would only obscure the access set"
)]
pub fn dispatch_fire(
    mut requests: MessageReader<FireRequested>,
    mut shooter_set: ParamSet<(ShooterQuery, TurnQuery)>,
    mut targets: TargetQuery,
    // GTW-323 / ADR-0004: the disjoint worn-armor relationship queries `fire()` resolves
    // a struck piece through (`ganger → Wears → the BodyPart-tagged piece`). `wears` reads
    // `&Wears` on gangers (a different component than `targets`' set); `pieces` reads+wears
    // the piece entities (a different entity set) — so neither conflicts with the
    // ShooterQuery/TurnQuery/TargetQuery access (no ParamSet needed).
    wears: WearsQuery,
    mut pieces: PieceQuery,
    // GTW-323 slice 2 / ADR-0004: the disjoint wielded-weapon relationship queries `fire()`
    // resolves the shooter's weapon through (`ganger → Wields → the weapon entity`).
    // `wields` reads `&Wields` on gangers (a different component than the shooter set);
    // `weapons` reads the weapon stats + decrements the `Magazine` on the weapon entities
    // (a different entity set) — so neither conflicts with the shooter/turn/target access.
    wields: WieldsQuery,
    mut weapons: WeaponQuery,
    mut grids: BattleGridsParam,
    tuning: Res<CombatTuning>,
    mut rng: ResMut<SimRng>,
    mut signals: FireSignals,
) {
    for request in requests.read() {
        // (1) READ the arc-relevant shooter state through the ShooterQuery half, copying
        //     every Copy value out so the query borrow ends at the block boundary (freeing
        //     the ParamSet to lend p1 below). A shooter not in the query (despawned) fires
        //     nothing (fail-closed).
        let shooters = shooter_set.p0();
        let Ok(((position, facing, _, aiming, _, _, tu_max), tu)) = shooters.get(request.shooter)
        else {
            continue;
        };
        let actor_cell = actor_cell(position);
        let facing: Direction = **facing;
        let tu: Tu = *tu;
        let tu_max: TuMax = *tu_max;
        let aiming: Aiming = *aiming;

        // GTW-323 slice 2: the weapon's DamageType (GTW-306) now lives on the related
        // weapon entity (`ganger → Wields → the weapon entity`), read here so the
        // per-round ShotFired can carry it (pure exposure; no fire-result change). A
        // shooter wielding no weapon — or whose weapon entity is not in the weapon query
        // — fires nothing (fail-closed, the same outcome `fire()` reaches internally).
        let Some(weapon_entity) = wields.get(request.shooter).ok().and_then(Wields::weapon) else {
            continue;
        };
        // The WeaponQuery row is (base_spread, accuracy, kickback, fatal_bias, damage,
        // punch, shred, DAMAGE_TYPE, stable, magazine) — the 8th leaf is the DamageType.
        let Ok((_, _, _, _, _, _, _, damage_type, ..)) = weapons.get(weapon_entity) else {
            continue;
        };
        let damage: DamageType = *damage_type;

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

        // (3b) GTW-328: declare the shot for the combat-text LOG — ONCE per fire request
        //      that proceeds (the Reject arm `continue`d above, so a rejected/unaffordable
        //      shot logs nothing). Emitted BEFORE the shot rolls, carrying ONLY data the
        //      dispatch already holds: the shooter ref, the resolved target occupant at the
        //      aimed (cell, level) (a single O(1) occupancy peek — NOT a fresh raycast or
        //      re-resolve), and the request's mode kind. No RNG draw, no fire-result logic
        //      — the determinism property is untouched.
        let aim_cell_level = CellLevel::new(request.target_cell, request.target_level);
        let target = grids.occupant_at(aim_cell_level);
        signals.declarations.write(FireDeclaration::new(
            request.shooter,
            target,
            request.mode.kind,
        ));

        // (4) Run the landed verb ONCE (REUSED verbatim) — it spends the fire TU and
        //     resolves the shot. The volley's effects are the in-world mutations
        //     (TU / ammo / target surfaces) the presenter observes via change-detection.
        let order = FireOrder {
            mode:         &request.mode,
            target_cell:  request.target_cell,
            target_level: request.target_level,
        };
        let mut shooters = shooter_set.p0();
        let volley = fire(
            request.shooter,
            order,
            &mut shooters,
            &mut targets,
            &wears,
            &mut pieces,
            &wields,
            &mut weapons,
            grids.grids(),
            &tuning,
            &mut rng,
        );

        // (5) GTW-290 / GTW-306 / GTW-302: emit one ShotFired per ROUND fired, sourced from
        //     the round's already-computed ShotOutcome plus the weapon's DamageType read in
        //     step (1) AND the PARALLEL HitReport the volley already produced (no recompute,
        //     no fire-result change) — so a burst draws a tracer per round, each carrying
        //     the per-type FX selector and the round's damage/wound/severity/armor verdict
        //     the floating-combat-text presenter reads. `Volley::reports` and
        //     `Volley::shots` are parallel (`reports[i]`/`shots[i]` are the same fired
        //     round, both the clamped-burst length), so the zip pairs each round's geometry
        //     with its own report — every fired round therefore carries `Some(report)`. An
        //     empty (fail-closed) volley emits none.
        for (outcome, report) in volley.shots.iter().zip(volley.reports.iter()) {
            signals.shots.write(ShotFired::from_round(
                request.shooter,
                damage,
                outcome,
                *report,
            ));
            // (5b) GTW-364: the fire→deplete→message BRIDGE. A round that depleted a piece
            //      of cover's HP to zero carries the destroyed (cell, level) on its report
            //      (resolve_and_apply already spent the ledger's HP via deplete_cover); emit
            //      ONE CoverDestroyed per such round. `sync_destroyed_cover` folds it into the
            //      occupancy grid's append-only destroyed-cover set (the cell stops blocking)
            //      and `should_recompute_visibility` (GTW-341) re-reveals the opened sightline
            //      — both already wired, consuming this message. No re-resolve, no extra draw.
            if let Some(at) = report.cover_destroyed {
                signals.cover_destroyed.write(CoverDestroyed::new(at));
            }
        }
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
