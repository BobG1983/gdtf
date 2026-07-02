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
    acts::{injury::InjuryInflicted, request::FireRequested},
    cover::CoverLedger,
    fire::{
        BattleGrids, FireOrder, MeleeQuery, MountedQuery, PieceQuery, ShooterQuery, TargetQuery,
        WeaponQuery, WearsQuery, WieldsQuery, fire,
    },
    firing_arc::target_in_arc,
    ganger::{Aiming, Direction, Facing, Position, Tu, TuMax},
    injuries::{InjuryRegistry, InjuryTables},
    magazine::mode_tu_cost,
    metric::{Cell, CellLevel},
    occupancy::OccupancyGrid,
    occupancy_sync::{CoverDestroyed, GroundAccrued, SlabDestroyed},
    resolve_coarse::ShotKind,
    rng::{InjuryRng, SeverityRng, ShotRng},
    shot_fired::ShotFired,
    slab::{BraceStairCells, SlabLedger},
    surface::SurfaceGrid,
    tu::spend_tu,
    tuning::CombatTuning,
    weapon::{DamageType, ModeKind},
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

/// The change-driven world-grid resources [`dispatch_fire`] reads, bundled into one
/// [`SystemParam`] so the system's parameter list stays under clippy's argument-count gate
/// (the [`BattleGrids`] / [`FireOrder`] grouping precedent in `fire.rs`).
///
/// Grouping the cohesive grid `Res<…>` reads into one param keeps [`dispatch_fire`] at
/// seven parameters; the body assembles the borrow-based [`BattleGrids`] from these `Res`
/// reads via [`BattleGridsParam::grids`]. A transparent system-param bundle of existing
/// named world-state resources — not itself a wrapped domain scalar.
///
/// GTW-392: [`BraceStairCells`] is included so the terrain-brace gate in `resolve_round`
/// can consult the lower-endpoint stair-cell set without an additional system param.
#[derive(SystemParam)]
pub struct BattleGridsParam<'w> {
    /// The coarse 3D occupancy grid — the march's collision / occupant-band surface.
    occupancy:   Res<'w, OccupancyGrid>,
    /// The persistent floor/roof-slab + ground surface grid the march flies through.
    surface:     Res<'w, SurfaceGrid>,
    /// The model cover ledger — peeked (read) for the faced cell + the target cell's
    /// cover band, and **spent** (write) when a round strikes cover (GTW-364), so it is
    /// a [`ResMut`] now (the cover-hit depletion path writes the ledger's HP in place).
    cover:       ResMut<'w, CoverLedger>,
    /// The model slab ledger — **spent** (write) when a round strikes a floor/roof slab
    /// (GTW-365), so it is a [`ResMut`] too (the slab-hit depletion path writes the
    /// ledger's HP in place). The march reads slab *existence* from `surface` above.
    slab:        ResMut<'w, SlabLedger>,
    /// GTW-392: the lower-endpoint brace-stair-cell set — the terrain-brace gate reads
    /// this to decide whether a kneeling stair occupant earns the brace bonus.
    brace_cells: Res<'w, BraceStairCells>,
}

impl BattleGridsParam<'_> {
    /// Assemble the borrow-based [`BattleGrids`] [`fire`] reads (and, GTW-364 / GTW-365,
    /// the cover- / slab-hit paths WRITE) from these grid resources — the occupancy /
    /// surface are read (`Res<T>` derefs to `&T`), the cover + slab ledgers are taken
    /// `&mut` (the depletion paths). Takes `&mut self` for the mutable ledger borrows;
    /// the grids are still never rebuilt — only the struck surface's HP is spent in place.
    fn grids(&mut self) -> BattleGrids<'_> {
        BattleGrids {
            occupancy:   &self.occupancy,
            surface:     &self.surface,
            cover:       &mut self.cover,
            slab:        &mut self.slab,
            brace_cells: &self.brace_cells,
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
/// the per-request [`FireDeclaration`] combat-log signal (GTW-328), the per-round
/// [`CoverDestroyed`] / [`SlabDestroyed`] destruction signals (GTW-364 / GTW-365), and the
/// per-round [`GroundAccrued`] accrual signal (GTW-366) — the fire→message bridges a
/// structural-hit / ground-hit round emits, which the maintenance + visibility systems
/// consume to free the smashed cell / accrue the ground damage. A transparent system-param
/// bundle of named output buffers — not itself a wrapped domain value.
#[derive(SystemParam)]
pub struct FireSignals<'w, 's> {
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
    /// The per-ROUND slab-destroyed signal (GTW-365) — emitted for each round whose
    /// [`HitReport::slab_destroyed`](crate::resolve_and_apply::HitReport::slab_destroyed)
    /// is `Some`, bridging the ledger's `deplete_slab` destruction into the buffered
    /// [`SlabDestroyed`] message that `sync_destroyed_slab` (sets the slab
    /// [`SlabState::Destroyed`](crate::surface::SlabState) on the surface grid) +
    /// `should_recompute_visibility` consume to stop blocking rounds + reopen LOS
    /// through the hole. The slab mirror of `cover_destroyed`.
    slab_destroyed:  MessageWriter<'w, SlabDestroyed>,
    /// The per-ROUND ground-accrued signal (GTW-366) — emitted for each round whose
    /// [`HitReport::ground_accrued`](crate::resolve_and_apply::HitReport::ground_accrued)
    /// is `Some`, bridging the round's `weapon_damage` into the buffered
    /// [`GroundAccrued`] message that `sync_accrued_ground` accrues (monotonically) onto
    /// the [`SurfaceGrid`](crate::surface::SurfaceGrid)'s per-cell ground accumulator. The
    /// ground-accrual mirror of `slab_destroyed`: the ground is damaged-never-destroyed, so
    /// this signal carries an accrual, not a destruction (purely cosmetic — crater FX is a
    /// later ticket).
    ground_accrued:  MessageWriter<'w, GroundAccrued>,
    /// The per-ROUND injury signal (GTW-438) — emitted for each round whose
    /// [`HitReport::injury`](crate::resolve_and_apply::HitReport::injury) is `Some`,
    /// bridging the in-fold injury roll into the buffered [`InjuryInflicted`] message
    /// [`apply_injury`](crate::acts::apply_injury) drains (and the presenter — GTW-439 —
    /// reads for the FCT / log flash). The injury-table mirror of the cover/slab/ground
    /// bridges: a structural hit destroys/accrues, a ganger wound INJURES.
    injuries:        MessageWriter<'w, InjuryInflicted>,
    /// The per-ROUND DOT-applied signal (GTW-544) — emitted for each round whose
    /// [`HitReport::dot_applied`](crate::resolve_and_apply::HitReport::dot_applied) is
    /// `Some` (a penetrating hit from a DOT weapon), bridging the in-fold attach decision
    /// into the buffered [`DotApplied`](crate::acts_runtime::dot::DotApplied) message
    /// [`apply_dot`](crate::acts_runtime::dot::apply_dot) drains (attaching or REFRESHING the
    /// [`Dot`](crate::weapon::Dot) on the struck ganger). The DOT mirror of the injury
    /// bridge: a ganger wound that penetrated from a DOT weapon AFFLICTS.
    dots:            MessageWriter<'w, crate::acts_runtime::dot::DotApplied>,
    /// The per-FIRE-ACT shove signal (GTW-525) — emitted ONCE when a `shove`-tagged weapon's
    /// shot CONNECTS with a ganger (the first connecting-ganger round of the volley). It
    /// writes an internal [`ShoveRequested`](crate::acts::request::ShoveRequested)
    /// (`ShoveSource::Weapon`) `dispatch_shove` drains the same frame (`dispatch_shove` is
    /// ordered `.after(dispatch_fire)`). A miss / a non-`shove` weapon writes nothing. Folded
    /// into this bundle so `dispatch_fire` stays under Bevy's 16-param limit.
    shoves:          MessageWriter<'w, crate::acts::request::ShoveRequested>,
    /// The firing weapon's [`Shove`](crate::weapon::Shove) tag read (GTW-525) — read off the
    /// resolved ranged-weapon entity to decide whether a connecting shot auto-shoves. A
    /// read-only [`Query`] over the weapon entities, folded into this bundle (with the `shoves`
    /// writer) so `dispatch_fire` stays under the 16-param limit; disjoint from the ganger /
    /// magazine queries (a read on a different component set).
    shove_tags:      Query<'w, 's, &'static crate::weapon::Shove>,
}

/// The two wielded-weapon MARKER probes [`dispatch_fire`] resolves a shooter's PREFERRED ranged
/// weapon through, bundled into one [`SystemParam`] so the system stays under Bevy's 16-param
/// limit (the [`BattleGridsParam`] grouping precedent).
///
/// Both are cheap unit-item archetype-filter probes over the weapon entities: [`MeleeQuery`]
/// (GTW-505 — EXCLUDES the ganger's melee weapon from the ranged resolution) and [`MountedQuery`]
/// (GTW-543 — the emplacement's bolted-down gun the manning ganger PREFERS). Disjoint from the
/// stat-reading [`WeaponQuery`] and each other, so no `ParamSet` is needed. `dispatch_fire`
/// resolves the weapon as `mounted → ranged` (prefer the mount, else the carried gun) and threads
/// both borrows into [`fire`] (which does the same internally).
#[derive(SystemParam)]
pub struct WeaponProbes<'w, 's> {
    /// The melee-weapon marker probe (GTW-505 C5) — the ranged resolution EXCLUDES a match.
    melee:   MeleeQuery<'w, 's>,
    /// The mounted-weapon marker probe (GTW-543) — the ranged resolution PREFERS a match.
    mounted: MountedQuery<'w, 's>,
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
///
/// GTW-70: made `pub` (with its decider [`decide_fire_arc`] and the [`can_engage`]
/// boolean wrapper) so the enemy-AI engagement gate and [`dispatch_fire`] share the ONE
/// arc verdict — neither re-derives it. The AI consults [`can_engage`] (the `¬Reject`
/// boolean) to decide whether a target is shootable; the dispatcher matches the full enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FireArcDecision {
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
///
/// GTW-70: `pub` so the enemy-AI engagement gate ([`crate::ai`]) and [`dispatch_fire`]
/// share the ONE arc verdict. The AI usually calls the [`can_engage`] boolean wrapper; the
/// dispatcher matches the full [`FireArcDecision`] (it needs the turn cost / facing).
#[must_use]
pub fn decide_fire_arc(
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

/// Whether a shooter at `actor_cell` (facing `facing`, pool `tu`) can **engage** a target
/// at `target_cell` under the GTW-242 firing-arc gate — `true` iff the arc verdict is NOT
/// [`FireArcDecision::Reject`] (GTW-70).
///
/// The boolean shape of [`decide_fire_arc`] the enemy-AI engagement gate ([`crate::ai`])
/// shares with [`dispatch_fire`]: an in-arc shot ([`FireArcDecision::FireInArc`]) and an
/// out-of-arc-but-affordable shot ([`FireArcDecision::TurnThenFire`]) both return `true`;
/// only the unaffordable out-of-arc reject returns `false`. Load-bearing for the AI's
/// turn-termination guarantee: the AI emits a `FireRequested` ONLY when this is `true`, so
/// the dispatcher can never silently reject the shot (spend no TU) and let the AI re-emit
/// it forever. Pure, total — a thin `matches!` over the SAME verdict the dispatcher runs.
#[must_use]
pub fn can_engage(
    facing: Direction,
    actor_cell: Cell,
    target_cell: Cell,
    tu: Tu,
    fire_cost: Tu,
    tuning: &CombatTuning,
) -> bool {
    !matches!(
        decide_fire_arc(facing, actor_cell, target_cell, tu, fire_cost, tuning),
        FireArcDecision::Reject
    )
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
    // GTW-505 C5 + GTW-543: the two wielded-weapon marker probes, grouped (WeaponProbes) so the
    // system stays under Bevy's 16-param limit. `melee` (GTW-505) EXCLUDES the ganger's melee
    // weapon from the ranged resolution; `mounted` (GTW-543) PREFERS the emplacement's bolted-down
    // gun the manning ganger fires. Both cheap unit-item probes over the weapon entities, disjoint
    // from `weapons` (which filters `With<WieldedBy>` and reads the stat columns), so no ParamSet.
    probes: WeaponProbes,
    mut grids: BattleGridsParam,
    tuning: Res<CombatTuning>,
    // GTW-14: disjoint per-subsystem RNG resources. ShotRng drives cone-sample +
    // body-part-roll; SeverityRng drives the §6 severity term. Two distinct
    // ResMut<T> are disjoint Bevy params (different resource types), so the
    // scheduler can parallelize this system against systems on other streams.
    // No system may take Res<ShotRng> or Res<SeverityRng> — see rng::streams doc.
    mut shot_rng: ResMut<ShotRng>,
    mut severity_rng: ResMut<SeverityRng>,
    // GTW-438: the injury-roll inputs threaded into `fire()`. `InjuryTables` /
    // `InjuryRegistry` are read (the weighted-pick table + the name→def resolution); they
    // are app/Load-OWNED resources (NOT inserted by the sim's `setup_battle`, unlike the
    // RNG streams), so a sim-only headless harness that opens a battle WITHOUT the Load
    // flow has neither — hence `Option<Res<…>>` + an empty-default fallback (bevy-traps.md
    // #1: a missing battle-lifetime resource must not panic a runtime system). With them
    // absent the roll finds no bucket and inflicts no injury, but STILL takes its one
    // InjuryRng draw (content-independent stream alignment). `InjuryRng` itself IS sim-set
    // (inserted by `setup_battle` alongside the other four streams), so it is a required
    // `ResMut`.
    injury_tables: Option<Res<InjuryTables>>,
    injury_registry: Option<Res<InjuryRegistry>>,
    mut injury_rng: ResMut<InjuryRng>,
    mut signals: FireSignals,
) {
    // Empty fallbacks for an asset-less harness (no Load flow → no InjuryTables/Registry).
    // A `Res` derefs to `&T`; an absent one falls back to a freshly-built empty default,
    // so `fire()` always gets a valid `&InjuryTables` / `&InjuryRegistry` to roll against
    // (the roll then finds no bucket but still takes its one draw).
    let empty_tables = InjuryTables::default();
    let empty_registry = InjuryRegistry::default();
    let tables: &InjuryTables = injury_tables.as_deref().unwrap_or(&empty_tables);
    let registry: &InjuryRegistry = injury_registry.as_deref().unwrap_or(&empty_registry);
    for request in requests.read() {
        // (1) READ the arc-relevant shooter state through the ShooterQuery half, copying
        //     every Copy value out so the query borrow ends at the block boundary (freeing
        //     the ParamSet to lend p1 below). A shooter not in the query (despawned) fires
        //     nothing (fail-closed).
        let shooters = shooter_set.p0();
        // The trailing `_` ignores the GTW-526 `Option<&Suppressed>` group member — the
        // arc-check read needs only pos/facing/aiming/tu_max; suppression enters the shot
        // math through the composer (`stability_for`), not this dispatch-arc gate.
        let Ok(((position, facing, _, aiming, _, _, tu_max, _), _, tu)) =
            shooters.get(request.shooter)
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
        // GTW-505 C5: resolve the RANGED weapon (excluding the melee weapon the ganger
        // also wields) so the ShotFired carries the GUN's DamageType, never the melee
        // weapon's — the same ranged-filtered resolution `fire()` does internally.
        // GTW-543: PREFER the emplacement's mounted gun (the ganger is manning it) over its own
        // carried gun, so the ShotFired carries the MOUNTED gun's DamageType while occupied — the
        // same mounted-preferring resolution `fire()` does internally.
        let Some(weapon_entity) = wields.get(request.shooter).ok().and_then(|w| {
            w.mounted_weapon(|entity| probes.mounted.get(entity).is_ok())
                .or_else(|| w.ranged_weapon(|entity| probes.melee.get(entity).is_ok()))
        }) else {
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
            &probes.melee,
            &probes.mounted,
            grids.grids(),
            &tuning,
            &mut shot_rng,
            &mut severity_rng,
            tables,
            registry,
            &mut injury_rng,
        );

        // (5) Emit the per-round output signals — the ShotFired FX/FCT, the injury bridge
        //     (GTW-438), and the structural cover/slab/ground bridges (GTW-364/365/366) —
        //     all PURE EXPOSURE of the volley the fire already produced (no recompute, no
        //     extra draw). Extracted to keep dispatch_fire under clippy's line gate.
        emit_round_signals(request.shooter, damage, &volley, &mut signals);

        // (6) GTW-525 C3: the RANGED weapon-tag auto-shove. If the firing weapon carries the
        //     `shove` tag AND a round CONNECTED with a ganger, knock that target back one cell
        //     (in addition to the shot's damage above). Write ONE internal ShoveRequested
        //     (ShoveSource::Weapon) for the FIRST connecting-ganger round — the connect already
        //     gated + the fire TU was charged, so dispatch_shove resolves it un-gated / TU-free;
        //     it is ordered `.after(dispatch_fire)`, so this same-frame message is consumed this
        //     tick. A MISS (no ShotKind::Ganger round) shoves nothing; a non-`shove` weapon
        //     shoves nothing. One shove per fire act (a burst does not multiply the knock-back).
        let weapon_shoves = signals.shove_tags.get(weapon_entity).is_ok_and(|tag| **tag);
        let struck_ganger = volley.shots.iter().find_map(|outcome| match outcome.kind {
            ShotKind::Ganger(target) => Some(target),
            _ => None,
        });
        if let (true, Some(struck)) = (weapon_shoves, struck_ganger) {
            signals
                .shoves
                .write(crate::acts::request::ShoveRequested::new_weapon(
                    request.shooter,
                    struck,
                ));
        }
    }
}

/// Emit the per-ROUND output signals for a resolved `volley` — the GTW-290/302 [`ShotFired`]
/// FX/FCT, the GTW-438 injury bridge, and the GTW-364/365/366 cover/slab/ground bridges.
///
/// One signal set per fired round, zipping the parallel
/// [`Volley::shots`](crate::fire::Volley::shots) geometry with the
/// [`Volley::reports`](crate::fire::Volley::reports) verdicts (`shots[i]`/`reports[i]` are
/// the same round). Every emission is PURE EXPOSURE of what the volley already computed —
/// no recompute, no fire-result change, no extra RNG draw (the injury roll happened in-fold,
/// frozen on `report.injury`). Extracted from [`dispatch_fire`] so that system stays under
/// clippy's line-count gate; takes the writer bundle by `&mut` (the [`MessageWriter`]s).
fn emit_round_signals(
    shooter: Entity,
    damage: DamageType,
    volley: &crate::fire::Volley,
    signals: &mut FireSignals,
) {
    for (outcome, report) in volley.shots.iter().zip(volley.reports.iter()) {
        // (5a) GTW-438: the injury bridge — a round that wounded a ganger with a non-graze,
        //      non-fatal severity AND rolled a named injury carries it on `report.injury`
        //      (the in-fold `roll_injury` already took its ONE InjuryRng draw); emit ONE
        //      InjuryInflicted per such round, addressed to the struck ganger entity on
        //      `report.kind`. `apply_injury` drains it (folds the GainedInjury into the
        //      target's InflictedInjuries + syncs the bleed); the presenter (GTW-439) reads
        //      it for the FCT/log flash. Cloned out BEFORE the ShotFired clone below.
        if let (Some(rolled), ShotKind::Ganger(target)) = (&report.injury, report.kind) {
            signals
                .injuries
                .write(InjuryInflicted::from_rolled(target, rolled.clone()));
        }
        // (5a2) GTW-544: the DOT bridge — a round that PENETRATED armor from a DOT weapon
        //       carries the Dot to attach on `report.dot_applied`; emit ONE DotApplied per
        //       such round, addressed to the struck ganger on `report.kind`. `apply_dot`
        //       attaches (or REFRESHES — refresh-not-stack) it. A fully-soaked hit / non-DOT
        //       weapon carries None → no message (the identity property).
        if let (Some(dot), ShotKind::Ganger(target)) = (report.dot_applied, report.kind) {
            signals
                .dots
                .write(crate::acts_runtime::dot::DotApplied::new(target, dot));
        }
        // The HitReport is non-`Copy` (it carries the rolled injury); clone it into the
        // per-round ShotFired (the FCT presenter reads the damage/wound/severity verdict —
        // the injury rides the separate InjuryInflicted).
        signals.shots.write(ShotFired::from_round(
            shooter,
            damage,
            outcome,
            report.clone(),
        ));
        // (5b) GTW-364: the cover fire→deplete→message bridge — a round that depleted cover
        //      to zero carries the destroyed (cell, level); emit one CoverDestroyed.
        if let Some(at) = report.cover_destroyed {
            signals.cover_destroyed.write(CoverDestroyed::new(at));
        }
        // (5c) GTW-365: the slab mirror — a round that depleted a slab to zero carries the
        //      destroyed (cell, level); emit one SlabDestroyed.
        if let Some(at) = report.slab_destroyed {
            signals.slab_destroyed.write(SlabDestroyed::new(at));
        }
        // (5d) GTW-366: the ground-accrual bridge — a round that struck the ground carries
        //      its cell + weapon_damage; emit one GroundAccrued (the ground is
        //      damaged-never-destroyed — accrual, not destruction).
        if let Some(accrual) = report.ground_accrued {
            signals
                .ground_accrued
                .write(GroundAccrued::new(accrual.cell, accrual.amount));
        }
    }
    // (5e) GTW-541 (`AoE` CORE of GTW-41): the SPLASH injury bridge. A non-Single round's
    //      template covers OTHER occupants (its blast / cone / line); each was applied to
    //      the world through the SAME resolve_and_apply path in-fold (HP / wounds already
    //      mutated). Bridge each splashed ganger's rolled named injury exactly as the
    //      primary report is bridged above (mirroring 5a) so the splash victim's injury
    //      lands too. `volley.splash` is EMPTY for a Single volley, so this loop is a no-op
    //      on the unchanged single-target path (the identity property). No RNG draw / no
    //      recompute — pure exposure of the frozen splash reports.
    for round_splash in &volley.splash {
        for report in round_splash {
            if let (Some(rolled), ShotKind::Ganger(target)) = (&report.injury, report.kind) {
                signals
                    .injuries
                    .write(InjuryInflicted::from_rolled(target, rolled.clone()));
            }
            // GTW-544: bridge each splashed ganger's DOT attach exactly as the primary
            // report is bridged above — a penetrating AoE splash from a DOT weapon afflicts
            // its splash victims too. Empty for a Single volley (the identity property).
            if let (Some(dot), ShotKind::Ganger(target)) = (report.dot_applied, report.kind) {
                signals
                    .dots
                    .write(crate::acts_runtime::dot::DotApplied::new(target, dot));
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
