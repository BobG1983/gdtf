//! The message-driven **input contract** for the landed combat acts + the per-act
//! **dispatch systems** that consume it + the public [`SimActsPlugin`] that registers
//! both — the SIM SIDE of E10's headless boundary (E10.2 / GTW-204).
//!
//! E10 drives the authoritative sim from BUFFERED Bevy MESSAGES, not direct calls: a
//! later CP437-input slice (out of scope here) emits a `*Requested` message per player
//! act, and ONE dispatch system per act drains its [`MessageReader`], fetches the
//! actor/target components via Bevy queries, and calls the ALREADY-LANDED verb once per
//! message. No act logic is reimplemented here — every dispatch system REUSES the
//! landed verb ([`fire`](crate::fire::fire) / [`set_aiming`](crate::posture::set_aiming)
//! / [`set_stance`](crate::posture::set_stance) / [`set_facing`](crate::posture::set_facing)
//! / [`stabilize_downed`](crate::downed_acts::stabilize_downed) /
//! [`execute_downed`](crate::downed_acts::execute_downed)) and its query/bundle shapes
//! verbatim.
//!
//! ## The `*Requested` types (the input contract)
//!
//! Seven [`#[derive(Message)]`](bevy::prelude::Message) buffered messages — mirroring
//! [`crate::bleed::Bleeding`] / [`crate::occupancy_sync::CoverDestroyed`], the buffered
//! `Message` API, NOT the observer `Event` API (`bevy-traps.md` #4). Each carries the
//! act's [`Entity`] actor ref(s) plus the act's OWNED payload. A `Message` cannot hold a
//! borrow, so [`FireRequested`] carries an OWNED [`FireModeSpec`] (it is `Copy`) plus the
//! target [`Cell`] / [`Level`] — the type has **no lifetime parameter**; the
//! [`dispatch_fire`] system reconstructs the borrow-based [`FireOrder`] `{ mode:
//! &owned_spec, target_cell, target_level }` from the owned payload at the call site.
//!
//! These carry [`Entity`] actor refs (matching the landed `fire(shooter: Entity)` and
//! the downed verbs' actor/target entities) — NOT presenter-facing integer ids. A
//! `*Resolved` integer-id boundary is a LATER epic; E10 relies on component
//! change-detection for the view, so this slice ships no `*Resolved` types. The
//! [`MoveRequested`] movement act (the seventh, added in GTW-234) carries the OWNED
//! [`CellLevel`] destination and runs the landed [`move_ganger`] verb — the move's TU
//! cost is the destination cell's terrain movement cost.
//!
//! ## [`SimActsPlugin`] (the registration unit)
//!
//! This slice CREATES the public [`SimActsPlugin`] (E10.0 lands only the
//! [`SimSystems::Simulate`] [`SystemSet`](bevy::prelude::SystemSet) enum + its
//! `configure_sets`; it builds no plugin). In `build()` the plugin
//! [`add_message`](bevy::app::App::add_message)s all seven types exactly once each
//! (`bevy-traps.md` #5) and adds the seven dispatch systems `.in_set(SimSystems::Simulate)`
//! in [`Update`](bevy::prelude::Update) — consuming E10.0's set (it imports and uses it,
//! never redefines it, and never calls `configure_sets`, which is E10.0's). So the
//! dispatch systems compose deterministically with the
//! [`OccupancyMaintenancePlugin`](crate::occupancy_sync::OccupancyMaintenancePlugin)
//! systems already in that set.
//!
//! Render-free, deterministic model logic: every random draw bottoms out in the single
//! injected [`SimRng`] resource (the fire dispatch system's `ResMut<SimRng>`); no
//! renderer, no window, no presenter, no pixel.

use bevy::{
    ecs::system::{ParamSet, SystemParam},
    prelude::{
        App, Deref, Entity, IntoScheduleConfigs, Message, MessageReader, Plugin, Query, Res,
        ResMut, Update,
    },
};

use crate::{
    cover::CoverLedger,
    downed_acts::{Actor, DownedTarget, execute_downed, stabilize_downed},
    fire::{BattleGrids, FireOrder, ShooterQuery, TargetQuery, fire},
    firing_arc::target_in_arc,
    ganger::{
        Aiming, Direction, Facing, Faction, LifeState, Position, Stabilized, Stance, StanceKind,
        Tu, TuMax,
    },
    magazine::mode_tu_cost,
    metric::{Cell, CellLevel, Level},
    move_acts::move_ganger,
    occupancy::OccupancyGrid,
    occupancy_sync::SimSystems,
    posture::{set_aiming, set_facing, set_stance},
    rng::SimRng,
    surface::SurfaceGrid,
    tu::spend_tu,
    tuning::CombatTuning,
    weapon::FireModeSpec,
};

/// The requested **aim flag** carried by a [`SetAimingRequested`] — `true` for aimed
/// fire, `false` for hip-fired.
///
/// A no-bare-types newtype over the aim-mode `bool` payload (a domain value — the
/// requested aim mode, not framework plumbing): a private inner + a derived [`Deref`],
/// the crate's newtype house style. Distinct from the [`Aiming`] *component* (the
/// ganger's current aim state): this is the *requested* value the dispatch verb sets the
/// component to.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AimRequest(bool);

impl AimRequest {
    /// Build a requested aim flag — `true` to aim, `false` to hip-fire.
    #[must_use]
    pub const fn new(aim: bool) -> Self {
        Self(aim)
    }
}

/// A **fire** act was requested — fire `mode` at `(target_cell, target_level)` for
/// `shooter`.
///
/// A buffered [`Message`] (`bevy-traps.md` #4 — NOT the observer `Event`), carrying the
/// [`Entity`] shooter ref plus the act's OWNED payload: a [`FireModeSpec`] (it is `Copy`,
/// so it is owned by value — a `Message` cannot hold a borrow) plus the target [`Cell`] /
/// [`Level`]. The type has **no lifetime parameter**; [`dispatch_fire`] reconstructs the
/// borrow-based [`FireOrder`] `{ mode: &mode, target_cell, target_level }` from this owned
/// payload. The shooter is a Bevy [`Entity`] handle — framework plumbing, the only bare
/// type the no-bare-types rule permits in a payload.
#[derive(Message, Debug, Clone, Copy, PartialEq)]
pub struct FireRequested {
    /// The firing entity (armed shooter).
    pub shooter:      Entity,
    /// The selected fire mode's per-mode numbers — OWNED (no borrow), so the message has
    /// no lifetime; the dispatch system borrows it into a [`FireOrder`].
    pub mode:         FireModeSpec,
    /// The target cell the player aimed at (the §2 aim cell's x/y).
    pub target_cell:  Cell,
    /// The target storey the player aimed at (the aim cell's z).
    pub target_level: Level,
}

impl FireRequested {
    /// Build a fire request for `shooter` firing `mode` at `(target_cell, target_level)`.
    #[must_use]
    pub const fn new(
        shooter: Entity,
        mode: FireModeSpec,
        target_cell: Cell,
        target_level: Level,
    ) -> Self {
        Self {
            shooter,
            mode,
            target_cell,
            target_level,
        }
    }
}

/// A **set-aiming** act was requested — set `actor`'s aim flag to `aim`.
///
/// A buffered [`Message`] carrying the [`Entity`] actor ref plus the requested
/// [`AimRequest`] flag. [`dispatch_set_aiming`] calls [`set_aiming`] (which spends NO TU —
/// toggling aim is free, `posture.rs`).
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SetAimingRequested {
    /// The acting ganger whose [`Aiming`] flag is set.
    pub actor: Entity,
    /// The requested aim value the flag is set to.
    pub aim:   AimRequest,
}

impl SetAimingRequested {
    /// Build a set-aiming request for `actor` to the requested aim flag.
    #[must_use]
    pub const fn new(actor: Entity, aim: AimRequest) -> Self {
        Self { actor, aim }
    }
}

/// A **set-stance** act was requested — change `actor`'s stance to `stance`.
///
/// A buffered [`Message`] carrying the [`Entity`] actor ref plus the requested
/// [`StanceKind`]. [`dispatch_set_stance`] calls [`set_stance`], which charges the
/// [`StanceChangeTu`](crate::tuning::StanceChangeTu) tuning leaf ONLY on a real change
/// (a no-op, no charge, when the actor already holds `stance`).
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SetStanceRequested {
    /// The acting ganger whose [`Stance`] is changed.
    pub actor:  Entity,
    /// The requested posture to change to.
    pub stance: StanceKind,
}

impl SetStanceRequested {
    /// Build a set-stance request for `actor` to change to `stance`.
    #[must_use]
    pub const fn new(actor: Entity, stance: StanceKind) -> Self {
        Self { actor, stance }
    }
}

/// A **set-facing** act was requested — turn `actor` to face `facing`.
///
/// A buffered [`Message`] carrying the [`Entity`] actor ref plus the requested
/// [`Direction`]. [`dispatch_set_facing`] calls [`set_facing`], which charges the
/// [`TurnTu`](crate::tuning::TurnTu) tuning leaf ONLY on a real turn (a no-op, no charge,
/// when the actor already faces `facing`).
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SetFacingRequested {
    /// The acting ganger whose [`Facing`] is turned.
    pub actor:  Entity,
    /// The requested direction to turn to.
    pub facing: Direction,
}

impl SetFacingRequested {
    /// Build a set-facing request for `actor` to turn to `facing`.
    #[must_use]
    pub const fn new(actor: Entity, facing: Direction) -> Self {
        Self { actor, facing }
    }
}

/// A **stabilize-downed** act was requested — `actor` stabilizes the downed `target`.
///
/// A buffered [`Message`] carrying the [`Entity`] actor + target refs.
/// [`dispatch_stabilize_downed`] assembles the [`Actor`] / [`DownedTarget`] bundles from
/// the queried components and calls [`stabilize_downed`], whose faction gate
/// ([`can_stabilize`](crate::downed_acts::can_stabilize)) holds end-to-end — only an
/// 8-adjacent alive ALLY sets the target's [`Stabilized`] flag (the target stays Downed).
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StabilizeDownedRequested {
    /// The acting (would-be stabilizer) ganger.
    pub actor:  Entity,
    /// The downed target to stabilize.
    pub target: Entity,
}

impl StabilizeDownedRequested {
    /// Build a stabilize-downed request for `actor` over `target`.
    #[must_use]
    pub const fn new(actor: Entity, target: Entity) -> Self {
        Self { actor, target }
    }
}

/// An **execute-downed** act was requested — `actor` executes the downed `target`.
///
/// A buffered [`Message`] carrying the [`Entity`] actor + target refs.
/// [`dispatch_execute_downed`] assembles the [`Actor`] / [`DownedTarget`] bundles from
/// the queried components and calls [`execute_downed`], whose faction gate
/// ([`can_execute`](crate::downed_acts::can_execute)) holds end-to-end — only an
/// 8-adjacent alive ENEMY transitions the target to [`LifeState::Dead`].
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ExecuteDownedRequested {
    /// The acting (would-be executor) ganger.
    pub actor:  Entity,
    /// The downed target to execute.
    pub target: Entity,
}

impl ExecuteDownedRequested {
    /// Build an execute-downed request for `actor` over `target`.
    #[must_use]
    pub const fn new(actor: Entity, target: Entity) -> Self {
        Self { actor, target }
    }
}

/// A **move** act was requested — step `actor` one cell to `dest`.
///
/// A buffered [`Message`] (`bevy-traps.md` #4 — NOT the observer `Event`), carrying the
/// [`Entity`] actor ref plus the destination [`CellLevel`] (the `(cell, level)` to step
/// to). The payload is OWNED and `Copy` ([`CellLevel`] is `Copy`), so the type has **no
/// lifetime parameter** — mirroring [`SetFacingRequested`] / [`SetStanceRequested`]. The
/// actor is a Bevy [`Entity`] handle — framework plumbing, the only bare type the
/// no-bare-types rule permits in a payload; `dest` is the landed [`CellLevel`] newtype.
/// [`dispatch_move`] drains this and runs the landed [`move_ganger`] verb once per
/// message, whose TU cost is the DESTINATION cell's terrain movement cost.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MoveRequested {
    /// The acting ganger to step.
    pub actor: Entity,
    /// The destination `(cell, level)` to step the actor to (one cell, no pathfinding).
    pub dest:  CellLevel,
}

impl MoveRequested {
    /// Build a move request for `actor` to step to `dest`.
    #[must_use]
    pub const fn new(actor: Entity, dest: CellLevel) -> Self {
        Self { actor, dest }
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
/// irrelevant to a ground facing). [`Position`] derefs to [`CellLevel`], which derefs to
/// the inner `IVec3`; the cell is its `x`/`y` (the [`crate::faced_cell`] split precedent).
fn actor_cell(position: &Position) -> Cell {
    let key = ***position;
    Cell::new(key.x, key.y)
}

/// **Dispatch** buffered [`SetAimingRequested`] messages — drain each and run the landed
/// [`set_aiming`] verb once per message (E10.2 AC4).
///
/// Queries the actor's [`Aiming`] component and calls [`set_aiming`] (charges NO TU —
/// toggling aim is free). REUSES the landed verb verbatim. A message for an actor without
/// an [`Aiming`] component is skipped (fail-closed, no panic).
pub fn dispatch_set_aiming(
    mut requests: MessageReader<SetAimingRequested>,
    mut actors: Query<&'static mut Aiming>,
) {
    for request in requests.read() {
        let Ok(mut aiming) = actors.get_mut(request.actor) else {
            continue;
        };
        set_aiming(&mut aiming, *request.aim);
    }
}

/// **Dispatch** buffered [`SetStanceRequested`] messages — drain each and run the landed
/// [`set_stance`] verb once per message (E10.2 AC4).
///
/// Queries the actor's [`Stance`] + [`Tu`] components plus the [`CombatTuning`] resource
/// and calls [`set_stance`] (charges [`StanceChangeTu`](crate::tuning::StanceChangeTu)
/// only on a real change — a no-op, no charge, when the actor already holds the requested
/// stance). REUSES the landed verb verbatim. A message for an actor missing either
/// component is skipped (fail-closed, no panic).
pub fn dispatch_set_stance(
    mut requests: MessageReader<SetStanceRequested>,
    mut actors: Query<(&'static mut Stance, &'static mut Tu)>,
    tuning: Res<CombatTuning>,
) {
    for request in requests.read() {
        let Ok((mut stance, mut tu)) = actors.get_mut(request.actor) else {
            continue;
        };
        set_stance(
            &mut stance,
            &mut tu,
            request.stance,
            &tuning.stance_change_tu,
        );
    }
}

/// **Dispatch** buffered [`SetFacingRequested`] messages — drain each and run the landed
/// [`set_facing`] verb once per message (E10.2 AC4).
///
/// Queries the actor's [`Facing`] + [`Tu`] components plus the [`CombatTuning`] resource
/// and calls [`set_facing`] (charges [`TurnTu`](crate::tuning::TurnTu) only on a real
/// turn — a no-op, no charge, when the actor already faces the requested direction).
/// REUSES the landed verb verbatim. A message for an actor missing either component is
/// skipped (fail-closed, no panic).
pub fn dispatch_set_facing(
    mut requests: MessageReader<SetFacingRequested>,
    mut actors: Query<(&'static mut Facing, &'static mut Tu)>,
    tuning: Res<CombatTuning>,
) {
    for request in requests.read() {
        let Ok((mut facing, mut tu)) = actors.get_mut(request.actor) else {
            continue;
        };
        set_facing(&mut facing, &mut tu, request.facing, &tuning.turn_tu);
    }
}

/// The downed-act read shape — the [`Position`] / [`LifeState`] / [`Faction`] every
/// from-Downed verb gates on (plus the target's [`Stabilized`] flag), queried off both
/// the actor and the target entity.
///
/// A type alias for the read tuple shared by [`dispatch_stabilize_downed`] /
/// [`dispatch_execute_downed`] so each system's signature stays readable: the [`Actor`] /
/// [`DownedTarget`] bundles ([`downed_acts`](crate::downed_acts)) are assembled from these
/// reads at the call site. The target's [`Stabilized`] is `Option` (it may be absent —
/// `None` is not-yet-stabilized). [`LifeState`] is `&mut` only on the target (execute
/// transitions it); the read shape is the same tuple for both actor and target via
/// `get`/`get_mut`.
type DownedReads<'world, 'state> = Query<
    'world,
    'state,
    (
        &'static Position,
        &'static mut LifeState,
        &'static Faction,
        Option<&'static mut Stabilized>,
    ),
>;

/// **Dispatch** buffered [`StabilizeDownedRequested`] messages — drain each and run the
/// landed [`stabilize_downed`] verb once per message (E10.2 AC5).
///
/// Reads the actor's [`Position`] / [`LifeState`] / [`Faction`] and the target's same
/// trio + [`Stabilized`] flag, assembles the [`Actor`] / [`DownedTarget`] bundles
/// ([`downed_acts`](crate::downed_acts)), and calls [`stabilize_downed`] — whose
/// faction gate ([`can_stabilize`](crate::downed_acts::can_stabilize)) holds end-to-end
/// (a cross-faction enemy is a no-op). On success it SETS the target's [`Stabilized`]
/// flag (the target stays [`LifeState::Downed`] — the verb never writes its life state).
/// REUSES the landed verb verbatim; an actor / target missing the read components or the
/// target missing its [`Stabilized`] component is skipped (fail-closed, no panic).
///
/// The actor and target reads are taken as snapshots (the gating reads are `Copy`), so
/// the verb's `&mut Stabilized` write to the target does not overlap a live read borrow.
pub fn dispatch_stabilize_downed(
    mut requests: MessageReader<StabilizeDownedRequested>,
    mut gangers: DownedReads,
    tuning: Res<CombatTuning>,
) {
    for request in requests.read() {
        // Snapshot the actor's gating reads (Copy newtypes), releasing the read borrow
        // before the target's &mut Stabilized write.
        let Ok((&actor_pos, &actor_life, &actor_faction, _)) = gangers.get(request.actor) else {
            continue;
        };
        // Snapshot the target's gating reads + its current Stabilized flag.
        let Ok((&target_pos, &target_life, &target_faction, target_stab)) =
            gangers.get(request.target)
        else {
            continue;
        };
        let actor = Actor {
            pos:     actor_pos,
            life:    actor_life,
            faction: actor_faction,
        };
        let target = DownedTarget {
            pos:        target_pos,
            life:       target_life,
            faction:    target_faction,
            stabilized: target_stab.copied(),
        };
        // Re-fetch the target's &mut Stabilized to apply the verb's write (the snapshot
        // borrows above are released — get_mut takes a fresh exclusive borrow).
        let Ok((_, _, _, Some(mut flag))) = gangers.get_mut(request.target) else {
            continue;
        };
        stabilize_downed(&actor, &target, &mut flag, &tuning);
    }
}

/// **Dispatch** buffered [`ExecuteDownedRequested`] messages — drain each and run the
/// landed [`execute_downed`] verb once per message (E10.2 AC5).
///
/// Reads the actor's and target's [`Position`] / [`LifeState`] / [`Faction`], assembles
/// the [`Actor`] / [`DownedTarget`] bundles ([`downed_acts`](crate::downed_acts)), and
/// calls [`execute_downed`] — whose faction gate
/// ([`can_execute`](crate::downed_acts::can_execute)) holds end-to-end (a same-faction
/// ally is a no-op). On success it transitions the target to [`LifeState::Dead`]. REUSES
/// the landed verb verbatim; an actor / target missing the read components is skipped
/// (fail-closed, no panic).
///
/// The actor and target gating reads are snapshotted (Copy), so the verb's `&mut
/// LifeState` write to the target does not overlap a live read borrow.
pub fn dispatch_execute_downed(
    mut requests: MessageReader<ExecuteDownedRequested>,
    mut gangers: DownedReads,
    tuning: Res<CombatTuning>,
) {
    for request in requests.read() {
        let Ok((&actor_pos, &actor_life, &actor_faction, _)) = gangers.get(request.actor) else {
            continue;
        };
        let Ok((&target_pos, &target_life, &target_faction, target_stab)) =
            gangers.get(request.target)
        else {
            continue;
        };
        let actor = Actor {
            pos:     actor_pos,
            life:    actor_life,
            faction: actor_faction,
        };
        let target = DownedTarget {
            pos:        target_pos,
            life:       target_life,
            faction:    target_faction,
            stabilized: target_stab.copied(),
        };
        // Re-fetch the target's &mut LifeState to apply the verb's write.
        let Ok((_, mut life, ..)) = gangers.get_mut(request.target) else {
            continue;
        };
        execute_downed(&actor, &target, &mut life, &tuning);
    }
}

/// **Dispatch** buffered [`MoveRequested`] messages — drain each and run the landed
/// [`move_ganger`] verb once per message (E4 / GTW-234).
///
/// Queries the actor's `(&mut `[`Position`]`, &mut `[`Tu`]`, &`[`LifeState`]`)`, reads
/// the [`OccupancyGrid`] (for the gates AND the destination-terrain cost lookup) and the
/// [`CombatTuning`] resource (for the [`MoveCosts`](crate::tuning::MoveCosts) table), and
/// calls [`move_ganger`] — whose gates (liveness / in-bounds / not-blocked / unoccupied /
/// affordable) hold end-to-end, charging the DESTINATION terrain's move cost. REUSES the
/// landed verb verbatim. A message for an actor missing any queried component is skipped
/// (fail-closed, no panic — the `dispatch_set_*` precedent).
///
/// This dispatch ONLY READS the grid (`Res<OccupancyGrid>`) for the gates + the terrain
/// cost; it never writes it. The grid's slot maintenance is the landed
/// [`sync_moved_gangers`](crate::occupancy_sync::sync_moved_gangers) reacting to the
/// `Changed<`[`Position`]`>` this verb produces — both sit in the
/// [`SimSystems::Simulate`] set and compose with no ambiguity (`dispatch_move`'s `&mut
/// Position` writes, `sync_moved_gangers`'s `&Position` reads it next).
pub fn dispatch_move(
    mut requests: MessageReader<MoveRequested>,
    mut actors: Query<(&'static mut Position, &'static mut Tu, &'static LifeState)>,
    grid: Res<OccupancyGrid>,
    tuning: Res<CombatTuning>,
) {
    for request in requests.read() {
        let Ok((mut position, mut tu, &life)) = actors.get_mut(request.actor) else {
            continue;
        };
        let _outcome = move_ganger(
            &mut position,
            &mut tu,
            life,
            request.dest,
            &grid,
            &tuning.move_costs,
        );
    }
}

/// The **sim-acts registration unit** — registers the seven `*Requested` message buffers
/// and adds the seven per-act dispatch systems, every one `.in_set(SimSystems::Simulate)`
/// in [`Update`] (E10.2 / GTW-204; the seventh — [`MoveRequested`] / [`dispatch_move`] —
/// added in GTW-234).
///
/// This slice CREATES this plugin — E10.0 lands only the [`SimSystems::Simulate`]
/// [`SystemSet`](bevy::prelude::SystemSet) enum and its `configure_sets`; it builds no
/// plugin. In `build()` the plugin:
///
/// - [`add_message`](App::add_message)s [`FireRequested`], [`SetAimingRequested`],
///   [`SetStanceRequested`], [`SetFacingRequested`], [`StabilizeDownedRequested`],
///   [`ExecuteDownedRequested`], and [`MoveRequested`] — exactly once each
///   (`bevy-traps.md` #5; an unregistered message buffer fails a [`MessageReader`]'s
///   param validation, the
///   [`OccupancyMaintenancePlugin`](crate::occupancy_sync::OccupancyMaintenancePlugin)
///   precedent); and
/// - adds the seven dispatch systems to [`Update`] `.in_set(SimSystems::Simulate)`,
///   composing deterministically with the occupancy-maintenance systems already in that
///   set (`bevy-traps.md` #3).
///
/// It consumes E10.0's [`SimSystems::Simulate`] set (it imports and uses it, never
/// redefines it) and does NOT call `configure_sets` — that is E10.0's job, run by
/// whichever plugin owns the set's configuration (the
/// [`OccupancyMaintenancePlugin`](crate::occupancy_sync::OccupancyMaintenancePlugin)).
/// Wiring this plugin into the `BattleRunning` lifecycle is E10.6, out of scope here.
#[derive(Debug, Default, Clone, Copy)]
pub struct SimActsPlugin;

impl Plugin for SimActsPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<FireRequested>()
            .add_message::<SetAimingRequested>()
            .add_message::<SetStanceRequested>()
            .add_message::<SetFacingRequested>()
            .add_message::<StabilizeDownedRequested>()
            .add_message::<ExecuteDownedRequested>()
            .add_message::<MoveRequested>()
            .add_systems(
                Update,
                (
                    dispatch_fire,
                    dispatch_set_aiming,
                    dispatch_set_stance,
                    dispatch_set_facing,
                    dispatch_stabilize_downed,
                    dispatch_execute_downed,
                    dispatch_move,
                )
                    .in_set(SimSystems::Simulate),
            );
    }
}

#[cfg(test)]
mod tests {
    use bevy::prelude::{App, Entity, MinimalPlugins, World};

    use super::*;
    use crate::{
        armor::{
            ArmorFloor, ArmorHardness, ArmorIntegrity, ArmorPiece, ArmorProtection, ArmorType,
            SourceArmor, WornArmor,
        },
        cover::{CoverEntry, CoverHp, HeightBand},
        ganger::{Hp, Luck, Shooting, Toughness, Wounds},
        magazine::Magazine,
        metric::{Cell, CellLevel, Level},
        occupancy_sync::OccupancyMaintenancePlugin,
        rng::BattleSeed,
        weapon::{
            Accuracy, BaseSpread, DamageProfile, DamageType, FatalBias, FireMode, HandlingProfile,
            Kickback, MagazineSize, ModeConeMult, ModeShots, ModeTuPercent, Stable, WeaponBundle,
            WeaponDamage, WeaponPunch, WeaponShred,
        },
    };

    /// A fixed seed for the per-test RNG stream (arbitrary, not tuned).
    const SEED: u64 = 0x5A1C_AC75;

    /// Insert the shared sim resources a dispatch system reads — the three grids, a
    /// seeded [`SimRng`], and [`CombatTuning::default`]. The grids are inserted EMPTY by
    /// default; a test mutates them via `app.world_mut()` before the run.
    fn insert_sim_resources(app: &mut App) {
        app.insert_resource(OccupancyGrid::new());
        app.insert_resource(SurfaceGrid::new());
        app.insert_resource(CoverLedger::new());
        app.insert_resource(SimRng::from_seed(BattleSeed::new(SEED)));
        app.insert_resource(CombatTuning::default());
    }

    /// Build a headless app: [`MinimalPlugins`] (no window / renderer) + [`SimActsPlugin`]
    /// + the shared sim resources — the `apply_hit` / `occupancy_sync` test precedent.
    fn headless_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(SimActsPlugin);
        insert_sim_resources(&mut app);
        app
    }

    /// A single-shot fire-mode spec from arbitrary (non-pinned) per-mode numbers.
    fn single_mode(tu_percent: f32, shots: u16) -> FireModeSpec {
        FireModeSpec::new(
            ModeConeMult::new(1.0),
            ModeTuPercent::new(tu_percent),
            ModeShots::new(shots),
        )
    }

    /// A worn suit whose every piece starts at the given stats — arbitrary (not shipped)
    /// magnitudes so a hit lands in a known regime.
    fn worn_suit(floor: i32, protection: i32, integrity: i32, hardness: i32) -> WornArmor {
        WornArmor::seed_from(&SourceArmor::uniform(ArmorPiece::new(
            ArmorFloor::new(floor),
            ArmorProtection::new(protection),
            ArmorIntegrity::new(integrity),
            ArmorHardness::new(hardness),
            ArmorType::DEFAULT,
        )))
    }

    /// Spawn an armed shooter facing East at `(x, y, 0)` — carries the full shooter-query
    /// component set AND the target-query set (the shooter is also a ganger, so its own
    /// liveness reads from the target query). Arbitrary magnitudes (not shipped tuning).
    fn spawn_shooter(
        world: &mut World,
        x: i32,
        y: i32,
        mode: FireModeSpec,
        aiming: bool,
    ) -> Entity {
        let mag_size = MagazineSize::new(30);
        let bundle = WeaponBundle::new(
            BaseSpread::new(0.05),
            Accuracy::new(2.0),
            Kickback::new(0.2),
            FatalBias::new(0.0),
            DamageProfile::new(
                WeaponDamage::new(40),
                WeaponPunch::new(20),
                WeaponShred::new(10),
                DamageType::Kinetic,
            ),
            HandlingProfile::new(
                mag_size,
                FireMode::Single { single: mode },
                Stable::new(true),
            ),
        );
        world
            .spawn((
                bundle,
                Position::new(CellLevel::new(Cell::new(x, y), Level::new(0))),
                Facing::new(Direction::East),
                Stance::new(StanceKind::Standing),
                Aiming::new(aiming),
                Shooting::new(1.0),
                Tu::new(200),
                crate::ganger::TuMax::new(100),
                Magazine::new(10, mag_size),
                Hp::new(50),
                Wounds::new(10),
                LifeState::Alive,
                worn_suit(0, 0, 1, 0),
                Toughness::new(1.0),
                Luck::new(0.0),
            ))
            .id()
    }

    /// The per-ganger battle-state bundle a fire target carries (the target query set +
    /// worn armor) — arbitrary magnitudes.
    fn target_bundle(hp: u16, wounds: u8, worn: WornArmor) -> impl bevy::prelude::Bundle {
        (
            Hp::new(hp),
            Wounds::new(wounds),
            LifeState::Alive,
            worn,
            Toughness::new(1.0),
            Luck::new(0.0),
        )
    }

    // === AC1 — the six `*Requested` types are constructible, carry their actor ref(s) +
    // owned payload, and `FireRequested` has no lifetime parameter. ===

    /// Mint a valid [`Entity`] handle for a test fixture (no `from_raw` in 0.18) — spawn
    /// an empty entity in a fresh world and take its id.
    fn an_entity() -> Entity {
        World::new().spawn_empty().id()
    }

    #[test]
    fn requested_types_construct_and_carry_their_refs_and_payload() {
        let shooter = an_entity();
        let target = an_entity();
        let mode = single_mode(0.2, 3);

        // FireRequested carries the shooter Entity + an OWNED FireModeSpec + Cell/Level
        // (no borrow → no lifetime parameter; that this compiles as `FireRequested`
        // without `<'_>` is the proof).
        let f = FireRequested::new(shooter, mode, Cell::new(8, 5), Level::new(0));
        assert_eq!(
            f.shooter, shooter,
            "FireRequested carries the shooter Entity"
        );
        assert_eq!(f.mode, mode, "FireRequested carries the OWNED FireModeSpec");
        assert_eq!(f.target_cell, Cell::new(8, 5), "carries the target Cell");
        assert_eq!(f.target_level, Level::new(0), "carries the target Level");

        // Posture requests carry one actor Entity + the requested kind.
        let a = SetAimingRequested::new(shooter, AimRequest::new(true));
        assert_eq!(a.actor, shooter);
        assert!(*a.aim, "the requested aim flag is carried");
        let s = SetStanceRequested::new(shooter, StanceKind::Prone);
        assert_eq!(s.actor, shooter);
        assert_eq!(s.stance, StanceKind::Prone);
        let fa = SetFacingRequested::new(shooter, Direction::West);
        assert_eq!(fa.actor, shooter);
        assert_eq!(fa.facing, Direction::West);

        // Downed requests carry BOTH actor + target entities.
        let st = StabilizeDownedRequested::new(shooter, target);
        assert_eq!(st.actor, shooter);
        assert_eq!(st.target, target);
        let ex = ExecuteDownedRequested::new(shooter, target);
        assert_eq!(ex.actor, shooter);
        assert_eq!(ex.target, target);

        // MoveRequested carries the actor Entity + an OWNED CellLevel destination (no
        // borrow → no lifetime parameter; that this compiles as `MoveRequested` without
        // `<'_>` is the no-lifetime proof).
        let dest = CellLevel::new(Cell::new(7, 3), Level::new(0));
        let mv = MoveRequested::new(shooter, dest);
        assert_eq!(mv.actor, shooter, "MoveRequested carries the actor Entity");
        assert_eq!(
            mv.dest, dest,
            "MoveRequested carries the OWNED CellLevel dest"
        );
    }

    // === AC2 — SimActsPlugin registers each `*Requested` buffer (a MessageReader passes
    // param-validation after update); the test reaches a post-update assert with no
    // validation panic. ===

    #[test]
    fn plugin_registers_every_message_buffer() {
        use bevy::prelude::{MessageReader, ResMut, Resource};

        /// A probe counter each reader-probe system bumps to prove it RAN (so its
        /// `MessageReader` param was validated by the scheduler, not skipped). One
        /// counter, not six bools, so all six probes increment the same resource.
        #[derive(Resource, Default)]
        struct Probed(u8);

        let mut app = headless_app();
        app.insert_resource(Probed::default());
        // One probe system per message type — each takes a `MessageReader<T>`, which the
        // scheduler param-validates against the registered buffer. An UNregistered buffer
        // would fail that validation; reaching the post-update assert (all six probes ran)
        // proves all six are registered by `SimActsPlugin`.
        app.add_systems(
            Update,
            (
                |mut r: MessageReader<FireRequested>, mut p: ResMut<Probed>| {
                    for _ in r.read() {}
                    p.0 += 1;
                },
                |mut r: MessageReader<SetAimingRequested>, mut p: ResMut<Probed>| {
                    for _ in r.read() {}
                    p.0 += 1;
                },
                |mut r: MessageReader<SetStanceRequested>, mut p: ResMut<Probed>| {
                    for _ in r.read() {}
                    p.0 += 1;
                },
                |mut r: MessageReader<SetFacingRequested>, mut p: ResMut<Probed>| {
                    for _ in r.read() {}
                    p.0 += 1;
                },
                |mut r: MessageReader<StabilizeDownedRequested>, mut p: ResMut<Probed>| {
                    for _ in r.read() {}
                    p.0 += 1;
                },
                |mut r: MessageReader<ExecuteDownedRequested>, mut p: ResMut<Probed>| {
                    for _ in r.read() {}
                    p.0 += 1;
                },
                |mut r: MessageReader<MoveRequested>, mut p: ResMut<Probed>| {
                    for _ in r.read() {}
                    p.0 += 1;
                },
            ),
        );

        app.update();

        let ran = app.world().get_resource::<Probed>().map(|p| p.0);
        assert_eq!(
            ran,
            Some(7),
            "every `MessageReader<*Requested>` must pass param-validation — all seven \
             buffers are registered by `SimActsPlugin`",
        );
    }

    // === AC3 — FireRequested dispatch runs fire(): a shot at an in-line target mutates
    // the target's battle surfaces (real shot effect); same seed reproduces it. ===

    /// Build the FIRE scenario in a fresh app: a shooter at (2,5) + a HIGH-band ganger
    /// target directly East at (8,5,0) in the occupancy grid. Returns `(app, shooter,
    /// target)`. Mirrors the fire.rs in-line target setup.
    fn fire_scenario() -> (App, Entity, Entity) {
        let mut app = headless_app();
        let mode = single_mode(0.2, 1);
        let shooter = spawn_shooter(app.world_mut(), 2, 5, mode, true);
        let target = app
            .world_mut()
            .spawn(target_bundle(30, 6, worn_suit(0, 0, 1, 0)))
            .id();
        let target_at = CellLevel::new(Cell::new(8, 5), Level::new(0));
        if let Some(mut grid) = app.world_mut().get_resource_mut::<OccupancyGrid>() {
            grid.set_occupant(target_at, Some(target));
            grid.set_occupant_band(target_at, Some(HeightBand::High));
        }
        (app, shooter, target)
    }

    #[test]
    fn fire_dispatch_runs_fire_and_mutates_the_target() {
        let (mut app, shooter, target) = fire_scenario();
        let mode = single_mode(0.2, 1);
        let hp_before = app.world().get::<Hp>(target).copied();
        let tu_before = app.world().get::<Tu>(shooter).copied();

        app.world_mut().write_message(FireRequested::new(
            shooter,
            mode,
            Cell::new(8, 5),
            Level::new(0),
        ));
        app.update();

        let hp_after = app.world().get::<Hp>(target).copied();
        let wounds_after = app.world().get::<Wounds>(target).map(|w| **w);
        let life_after = app.world().get::<LifeState>(target).copied();
        let tu_after = app.world().get::<Tu>(shooter).copied();

        // The verb RAN: either the target's battle surfaces changed (a real shot effect)
        // OR (seed-dependent miss) the shooter's Tu strictly decreased (the charge). Never
        // a pinned magnitude.
        let target_changed = hp_after != hp_before
            || wounds_after != Some(6)
            || life_after != Some(LifeState::Alive);
        let tu_dropped = matches!((tu_before, tu_after), (Some(b), Some(a)) if *a < *b);
        assert!(
            target_changed || tu_dropped,
            "fire dispatch must run the verb — target surfaces changed or the shooter's \
             Tu dropped (hp {hp_before:?}->{hp_after:?}, tu {tu_before:?}->{tu_after:?})",
        );
    }

    #[test]
    fn fire_dispatch_is_deterministic_for_the_same_seed() {
        let snapshot = |seed_run: u64| {
            // The seed is fixed by insert_sim_resources; seed_run only labels the call.
            let _ = seed_run;
            let (mut app, shooter, target) = fire_scenario();
            let mode = single_mode(0.2, 1);
            app.world_mut().write_message(FireRequested::new(
                shooter,
                mode,
                Cell::new(8, 5),
                Level::new(0),
            ));
            app.update();
            (
                app.world().get::<Hp>(target).map(|h| **h),
                app.world().get::<Wounds>(target).map(|w| **w),
                app.world().get::<LifeState>(target).copied(),
                app.world().get::<Tu>(shooter).map(|t| **t),
            )
        };
        assert_eq!(
            snapshot(0),
            snapshot(1),
            "the same BattleSeed must reproduce the same post-fire state via dispatch",
        );
    }

    // === GTW-242 — the firing-arc + turn-to-fire gate layered on the fire dispatch.
    // Value-agnostic on every magnitude: the assertions are RELATIONS (drop == fire_cost
    // in-arc; == turn_cost + fire_cost out-of-arc; UNCHANGED on reject), never pinned. ===

    /// Spawn an armed shooter facing `facing` at `(x, y, 0)` with a given starting `tu`
    /// pool — the GTW-242 fixture (reuses `spawn_shooter`, then overrides the facing + the
    /// pool so a test can place the shooter at the boundary of an affordability gate). The
    /// shooter is NOT aiming (hip-fire) so the fire cost has no aim premium muddying the
    /// relation. Returns the shooter [`Entity`].
    fn spawn_arc_shooter(world: &mut World, x: i32, y: i32, facing: Direction, tu: u8) -> Entity {
        let mode = single_mode(0.2, 1);
        let shooter = spawn_shooter(world, x, y, mode, false);
        if let Ok(mut entity) = world.get_entity_mut(shooter) {
            entity.insert((Facing::new(facing), Tu::new(tu)));
        }
        shooter
    }

    /// Place a HIGH-band ganger target at `(x, y, 0)` in the occupancy grid + spawn its
    /// battle-state bundle — the GTW-242 target fixture (mirrors `fire_scenario`).
    fn place_arc_target(app: &mut App, x: i32, y: i32) -> Entity {
        let target = app
            .world_mut()
            .spawn(target_bundle(30, 6, worn_suit(0, 0, 1, 0)))
            .id();
        let at = CellLevel::new(Cell::new(x, y), Level::new(0));
        if let Some(mut grid) = app.world_mut().get_resource_mut::<OccupancyGrid>() {
            grid.set_occupant(at, Some(target));
            grid.set_occupant_band(at, Some(HeightBand::High));
        }
        target
    }

    /// The fire-TU cost a hip-fired `single_mode(0.2, 1)` shot charges under the app's
    /// current [`CombatTuning`] (the same `mode_tu_cost` source `fire()` debits) — read from
    /// the resource so the relation tracks the tunable value, never a pinned magnitude.
    fn fire_cost_in(app: &App) -> Option<u8> {
        let tuning = app.world().get_resource::<CombatTuning>()?;
        let mode = single_mode(0.2, 1);
        Some(*mode_tu_cost(
            &mode,
            &TuMax::new(100),
            &Aiming::new(false),
            tuning,
        ))
    }

    /// The per-45°-step turn cost off the app's current [`CombatTuning`].
    fn turn_tu_in(app: &App) -> Option<u8> {
        app.world()
            .get_resource::<CombatTuning>()
            .map(|t| *t.turn_tu)
    }

    // GTW-242 AC1 — an IN-ARC shot (target dead ahead) spends only the fire TU and leaves
    // the facing unchanged. Shooter East at (5,5), target East at (8,5) — 0° off-axis.
    #[test]
    fn in_arc_shot_spends_only_fire_tu_and_keeps_facing() {
        let mut app = headless_app();
        let shooter = spawn_arc_shooter(app.world_mut(), 5, 5, Direction::East, 200);
        let _target = place_arc_target(&mut app, 8, 5);
        let fire_cost = fire_cost_in(&app);
        let tu_before = app.world().get::<Tu>(shooter).map(|t| **t);

        app.world_mut().write_message(FireRequested::new(
            shooter,
            single_mode(0.2, 1),
            Cell::new(8, 5),
            Level::new(0),
        ));
        app.update();

        let facing_after = app.world().get::<Facing>(shooter).map(|f| **f);
        let tu_after = app.world().get::<Tu>(shooter).map(|t| **t);
        assert_eq!(
            facing_after,
            Some(Direction::East),
            "an in-arc shot must NOT change the facing",
        );
        assert_eq!(
            tu_before.zip(tu_after).map(|(b, a)| b - a),
            fire_cost,
            "an in-arc shot drops TU by exactly the fire cost (no turn cost)",
        );
    }

    // GTW-242 AC2 — an OUT-OF-ARC shot with enough TU turns to face the target THEN fires:
    // the facing becomes from_cells(actor, target) and TU drops by exactly turn + fire.
    // Shooter East at (5,5), target South at (5,8) — 90° off a 120° (±60°) arc.
    #[test]
    fn out_of_arc_with_enough_tu_turns_then_fires() {
        let mut app = headless_app();
        let shooter = spawn_arc_shooter(app.world_mut(), 5, 5, Direction::East, 200);
        let _target = place_arc_target(&mut app, 5, 8);
        let fire_cost = fire_cost_in(&app);
        let turn_tu = turn_tu_in(&app);
        // The expected combined drop: steps_to(East -> South) * turn_tu + fire_cost.
        let expected_drop = fire_cost
            .zip(turn_tu)
            .map(|(f, t)| Direction::East.steps_to(Direction::South) * t + f);
        let tu_before = app.world().get::<Tu>(shooter).map(|t| **t);

        app.world_mut().write_message(FireRequested::new(
            shooter,
            single_mode(0.2, 1),
            Cell::new(5, 8),
            Level::new(0),
        ));
        app.update();

        let facing_after = app.world().get::<Facing>(shooter).map(|f| **f);
        let tu_after = app.world().get::<Tu>(shooter).map(|t| **t);
        assert_eq!(
            facing_after,
            Direction::from_cells(Cell::new(5, 5), Cell::new(5, 8)),
            "an out-of-arc shot must turn to face from_cells(actor, target) (South)",
        );
        assert_eq!(
            tu_before.zip(tu_after).map(|(b, a)| b - a),
            expected_drop,
            "an out-of-arc shot drops TU by exactly turn_cost + fire_cost",
        );
    }

    // GTW-242 AC3 (the crux) — an OUT-OF-ARC shot that can afford the FIRE but NOT
    // turn + fire is REJECTED: no TU spent, no facing change, no shot. Same geometry as
    // AC2; the pool is set to fire_cost + 1 < fire_cost + turn_cost (turn_cost == 2 here).
    #[test]
    fn out_of_arc_unaffordable_turn_is_rejected_no_spend_no_turn() {
        let mut app = headless_app();
        // The fire cost (the same value `fire()` debits) decides where the affordability
        // gap sits. Read it before spawning the shooter at the boundary inside it.
        let Some(fire_cost) = fire_cost_in(&app) else {
            // CombatTuning is always inserted by headless_app; the None arm is unreachable,
            // but a test must not unwrap/expect/panic — assert the precondition holds.
            assert!(
                app.world().get_resource::<CombatTuning>().is_some(),
                "tuning present",
            );
            return;
        };
        // fire_cost <= tu < fire_cost + turn_cost: afford the shot, NOT the turn+shot.
        // turn_cost (East -> South) == steps_to(2) * turn_tu(1) == 2, so fire_cost + 1 sits
        // strictly inside the gap.
        let tu_start = fire_cost + 1;
        let shooter = spawn_arc_shooter(app.world_mut(), 5, 5, Direction::East, tu_start);
        let target = place_arc_target(&mut app, 5, 8);
        let hp_before = app.world().get::<Hp>(target).map(|h| **h);
        let life_before = app.world().get::<LifeState>(target).copied();

        app.world_mut().write_message(FireRequested::new(
            shooter,
            single_mode(0.2, 1),
            Cell::new(5, 8),
            Level::new(0),
        ));
        app.update();

        let facing_after = app.world().get::<Facing>(shooter).map(|f| **f);
        let tu_after = app.world().get::<Tu>(shooter).map(|t| **t);
        let hp_after = app.world().get::<Hp>(target).map(|h| **h);
        let life_after = app.world().get::<LifeState>(target).copied();
        assert_eq!(
            facing_after,
            Some(Direction::East),
            "a rejected shot must NOT change the facing",
        );
        assert_eq!(
            tu_after,
            Some(tu_start),
            "a rejected shot must spend NO TU (pool unchanged)",
        );
        assert_eq!(
            (hp_after, life_after),
            (hp_before, life_before),
            "a rejected shot must resolve NO shot (target surfaces unchanged)",
        );
    }

    // GTW-242 AC5 — the arc is DATA-DRIVEN. A WIDE arc (360°) makes the (5,8) target in-arc
    // (no turn — facing unchanged, only fire_cost); a NARROW arc forces an otherwise-in-arc
    // off-axis target out-of-arc (now turns). Mutate CombatTuning before the request.
    #[test]
    fn arc_is_data_driven_wide_never_turns_narrow_forces_turn() {
        // (a) WIDE arc 360° — the 90°-off (5,8) target is in-arc: no turn, only fire_cost.
        let mut wide = headless_app();
        if let Some(mut t) = wide.world_mut().get_resource_mut::<CombatTuning>() {
            t.firing_arc = crate::tuning::FiringArc::new(360.0);
        }
        let s_wide = spawn_arc_shooter(wide.world_mut(), 5, 5, Direction::East, 200);
        let _t_wide = place_arc_target(&mut wide, 5, 8);
        let fire_cost = fire_cost_in(&wide);
        let tu_before = wide.world().get::<Tu>(s_wide).map(|t| **t);
        wide.world_mut().write_message(FireRequested::new(
            s_wide,
            single_mode(0.2, 1),
            Cell::new(5, 8),
            Level::new(0),
        ));
        wide.update();
        assert_eq!(
            wide.world().get::<Facing>(s_wide).map(|f| **f),
            Some(Direction::East),
            "a 360° arc makes every target in-arc — the facing must NOT change",
        );
        assert_eq!(
            tu_before
                .zip(wide.world().get::<Tu>(s_wide).map(|t| **t))
                .map(|(b, a)| b - a),
            fire_cost,
            "a 360° in-arc shot drops only the fire cost (no turn)",
        );

        // (b) NARROW arc 10° — a near-on-axis target (8,6, ~18° off East) is now OUT-of-arc
        // and must turn (facing changes to SouthEast = from_cells((5,5),(8,6))).
        let mut narrow = headless_app();
        if let Some(mut t) = narrow.world_mut().get_resource_mut::<CombatTuning>() {
            t.firing_arc = crate::tuning::FiringArc::new(10.0);
        }
        let s_narrow = spawn_arc_shooter(narrow.world_mut(), 5, 5, Direction::East, 200);
        let _t_narrow = place_arc_target(&mut narrow, 8, 6);
        narrow.world_mut().write_message(FireRequested::new(
            s_narrow,
            single_mode(0.2, 1),
            Cell::new(8, 6),
            Level::new(0),
        ));
        narrow.update();
        assert_eq!(
            narrow.world().get::<Facing>(s_narrow).map(|f| **f),
            Direction::from_cells(Cell::new(5, 5), Cell::new(8, 6)),
            "a narrow arc forces an off-axis target out-of-arc — the shooter must turn",
        );
    }

    // GTW-242 AC6 — a CO-LOCATED target (the shooter's own cell, zero vector) is in-arc: it
    // spends only the fire cost, leaves the facing unchanged, and never panics / NaNs.
    #[test]
    fn co_located_target_is_in_arc_only_fire_cost_no_panic() {
        let mut app = headless_app();
        let shooter = spawn_arc_shooter(app.world_mut(), 5, 5, Direction::North, 200);
        let _target = place_arc_target(&mut app, 5, 5);
        let fire_cost = fire_cost_in(&app);
        let tu_before = app.world().get::<Tu>(shooter).map(|t| **t);

        app.world_mut().write_message(FireRequested::new(
            shooter,
            single_mode(0.2, 1),
            Cell::new(5, 5),
            Level::new(0),
        ));
        app.update();

        assert_eq!(
            app.world().get::<Facing>(shooter).map(|f| **f),
            Some(Direction::North),
            "a co-located target needs no turn — the facing must NOT change",
        );
        assert_eq!(
            tu_before
                .zip(app.world().get::<Tu>(shooter).map(|t| **t))
                .map(|(b, a)| b - a),
            fire_cost,
            "a co-located in-arc shot drops only the fire cost (no turn, no NaN)",
        );
    }

    // === AC4 — posture dispatch runs the verbs with their TU semantics. ===

    #[test]
    fn set_stance_dispatch_changes_stance_and_spends_the_tuning_leaf() {
        let mut app = headless_app();
        let actor = app
            .world_mut()
            .spawn((Stance::new(StanceKind::Standing), Tu::new(60)))
            .id();
        let cost = app
            .world()
            .get_resource::<CombatTuning>()
            .map(|t| *t.stance_change_tu);
        let tu_before = app.world().get::<Tu>(actor).map(|t| **t);

        app.world_mut()
            .write_message(SetStanceRequested::new(actor, StanceKind::Prone));
        app.update();

        let stance_after = app.world().get::<Stance>(actor).map(|s| **s);
        let tu_after = app.world().get::<Tu>(actor).map(|t| **t);
        assert_eq!(
            stance_after,
            Some(StanceKind::Prone),
            "set-stance dispatch must change the stance to the requested value",
        );
        // The TU drop equals exactly the consulted stance_change_tu leaf (a relation to
        // the tuning value, never a pinned magnitude).
        assert!(
            matches!((tu_before, tu_after), (Some(b), Some(a)) if a < b),
            "a real stance change must strictly decrease Tu",
        );
        assert_eq!(
            tu_before.zip(tu_after).map(|(b, a)| b - a),
            cost,
            "the Tu drop must equal exactly the stance_change_tu tuning leaf",
        );
    }

    #[test]
    fn set_stance_dispatch_to_same_stance_is_a_no_op_on_tu() {
        let mut app = headless_app();
        let actor = app
            .world_mut()
            .spawn((Stance::new(StanceKind::Crouching), Tu::new(60)))
            .id();
        app.world_mut()
            .write_message(SetStanceRequested::new(actor, StanceKind::Crouching));
        app.update();
        assert_eq!(
            app.world().get::<Tu>(actor).map(|t| **t),
            Some(60),
            "re-asserting the held stance must not spend any TU (the verb's no-op)",
        );
        assert_eq!(
            app.world().get::<Stance>(actor).map(|s| **s),
            Some(StanceKind::Crouching),
            "the stance is unchanged on a no-op",
        );
    }

    #[test]
    fn set_facing_dispatch_changes_facing_and_spends_the_tuning_leaf() {
        let mut app = headless_app();
        let actor = app
            .world_mut()
            .spawn((Facing::new(Direction::North), Tu::new(50)))
            .id();
        // The per-step turn cost off the shipped tuning, scaled by the short-way step
        // count (North -> East is a 2-step turn) — the expected fully-affordable charge.
        let per_step = app
            .world()
            .get_resource::<CombatTuning>()
            .map(|t| *t.turn_tu);
        let expected_drop = per_step.map(|c| Direction::North.steps_to(Direction::East) * c);
        let tu_before = app.world().get::<Tu>(actor).map(|t| **t);

        app.world_mut()
            .write_message(SetFacingRequested::new(actor, Direction::East));
        app.update();

        assert_eq!(
            app.world().get::<Facing>(actor).map(|f| **f),
            Some(Direction::East),
            "set-facing dispatch must turn to the requested direction",
        );
        let tu_after = app.world().get::<Tu>(actor).map(|t| **t);
        assert!(
            matches!((tu_before, tu_after), (Some(b), Some(a)) if a < b),
            "a real turn must strictly decrease Tu",
        );
        assert_eq!(
            tu_before.zip(tu_after).map(|(b, a)| b - a),
            expected_drop,
            "the Tu drop must equal exactly (short-way steps) * the per-step turn_tu leaf",
        );
    }

    #[test]
    fn set_facing_dispatch_to_same_facing_is_a_no_op_on_tu() {
        let mut app = headless_app();
        let actor = app
            .world_mut()
            .spawn((Facing::new(Direction::SouthWest), Tu::new(50)))
            .id();
        app.world_mut()
            .write_message(SetFacingRequested::new(actor, Direction::SouthWest));
        app.update();
        assert_eq!(
            app.world().get::<Tu>(actor).map(|t| **t),
            Some(50),
            "re-asserting the held facing must not spend any TU (the verb's no-op)",
        );
    }

    #[test]
    fn set_aiming_dispatch_flips_the_flag_and_leaves_tu_unchanged() {
        let mut app = headless_app();
        let actor = app
            .world_mut()
            .spawn((Aiming::new(false), Tu::new(40)))
            .id();
        app.world_mut()
            .write_message(SetAimingRequested::new(actor, AimRequest::new(true)));
        app.update();
        assert_eq!(
            app.world().get::<Aiming>(actor).map(|a| **a),
            Some(true),
            "set-aiming dispatch must flip the aim flag to the requested value",
        );
        // Toggling aim is free — set_aiming spends no TU (posture.rs).
        assert_eq!(
            app.world().get::<Tu>(actor).map(|t| **t),
            Some(40),
            "toggling aim must NOT change Tu (aiming is free)",
        );
    }

    // === AC5 — downed dispatch runs the faction-gated verbs; the gate holds end-to-end.
    // ===

    /// Spawn a downed-act actor at `(x, y, 0)` of `faction`, [`LifeState::Alive`].
    fn spawn_downed_actor(world: &mut World, x: i32, y: i32, faction: u8) -> Entity {
        world
            .spawn((
                Position::new(CellLevel::new(Cell::new(x, y), Level::new(0))),
                LifeState::Alive,
                Faction::new(faction),
                Stabilized::new(false),
            ))
            .id()
    }

    /// Spawn a downed-act target at `(x, y, 0)` of `faction`, [`LifeState::Downed`], not
    /// yet stabilized.
    fn spawn_downed_target(world: &mut World, x: i32, y: i32, faction: u8) -> Entity {
        world
            .spawn((
                Position::new(CellLevel::new(Cell::new(x, y), Level::new(0))),
                LifeState::Downed,
                Faction::new(faction),
                Stabilized::new(false),
            ))
            .id()
    }

    #[test]
    fn stabilize_dispatch_from_adjacent_ally_sets_the_flag_and_keeps_downed() {
        let mut app = headless_app();
        let actor = spawn_downed_actor(app.world_mut(), 10, 10, 1);
        let target = spawn_downed_target(app.world_mut(), 11, 10, 1); // adjacent, same faction

        app.world_mut()
            .write_message(StabilizeDownedRequested::new(actor, target));
        app.update();

        assert_eq!(
            app.world().get::<Stabilized>(target).map(|s| **s),
            Some(true),
            "an adjacent alive ALLY's stabilize dispatch must SET the Stabilized flag",
        );
        assert_eq!(
            app.world().get::<LifeState>(target).copied(),
            Some(LifeState::Downed),
            "stabilize never writes LifeState — the target stays Downed",
        );
    }

    #[test]
    fn execute_dispatch_from_adjacent_enemy_kills_the_target() {
        let mut app = headless_app();
        let actor = spawn_downed_actor(app.world_mut(), 10, 10, 1);
        let target = spawn_downed_target(app.world_mut(), 11, 10, 2); // adjacent, enemy faction

        app.world_mut()
            .write_message(ExecuteDownedRequested::new(actor, target));
        app.update();

        assert_eq!(
            app.world().get::<LifeState>(target).copied(),
            Some(LifeState::Dead),
            "an adjacent alive ENEMY's execute dispatch must transition the target to Dead",
        );
    }

    #[test]
    fn faction_gate_holds_end_to_end_through_dispatch() {
        // A cross-faction (enemy) STABILIZE is a no-op (flag stays false).
        let mut app = headless_app();
        let enemy_actor = spawn_downed_actor(app.world_mut(), 10, 10, 1);
        let downed_enemy = spawn_downed_target(app.world_mut(), 11, 10, 2); // different faction
        app.world_mut()
            .write_message(StabilizeDownedRequested::new(enemy_actor, downed_enemy));
        app.update();
        assert_eq!(
            app.world().get::<Stabilized>(downed_enemy).map(|s| **s),
            Some(false),
            "a cross-faction (enemy) stabilize dispatch is a no-op — the flag stays false",
        );

        // A same-faction (ally) EXECUTE is a no-op (target stays Downed).
        let mut app2 = headless_app();
        let ally_actor = spawn_downed_actor(app2.world_mut(), 10, 10, 1);
        let downed_ally = spawn_downed_target(app2.world_mut(), 11, 10, 1); // same faction
        app2.world_mut()
            .write_message(ExecuteDownedRequested::new(ally_actor, downed_ally));
        app2.update();
        assert_eq!(
            app2.world().get::<LifeState>(downed_ally).copied(),
            Some(LifeState::Downed),
            "a same-faction (ally) execute dispatch is a no-op — the target stays Downed",
        );
    }

    // === AC6 — the empty-input invariant: with NO `*Requested` emitted, every dispatch
    // system is inert and mutates nothing across several updates. ===

    #[test]
    fn no_messages_means_no_mutation() {
        let (mut app, shooter, target) = fire_scenario();
        // Also a posture actor and a downed pair, so every dispatch system has a subject
        // it WOULD mutate if it ran spuriously.
        let posture_actor = app
            .world_mut()
            .spawn((
                Stance::new(StanceKind::Standing),
                Facing::new(Direction::North),
                Aiming::new(false),
                Tu::new(60),
            ))
            .id();
        let downed_actor = spawn_downed_actor(app.world_mut(), 20, 20, 1);
        let downed_target = spawn_downed_target(app.world_mut(), 21, 20, 1);
        // A move-capable actor (Position/Tu/LifeState) so dispatch_move has a subject it
        // WOULD mutate (Position + Tu) if it ran spuriously.
        let move_actor = app
            .world_mut()
            .spawn((
                Position::new(CellLevel::new(Cell::new(30, 30), Level::new(0))),
                Tu::new(80),
                LifeState::Alive,
            ))
            .id();

        // Snapshot every relevant component before any update.
        let snap = |app: &App| {
            (
                app.world().get::<Hp>(target).copied(),
                app.world().get::<Wounds>(target).copied(),
                app.world().get::<LifeState>(target).copied(),
                app.world().get::<Tu>(shooter).copied(),
                app.world().get::<Stance>(posture_actor).copied(),
                app.world().get::<Facing>(posture_actor).copied(),
                app.world().get::<Aiming>(posture_actor).copied(),
                app.world().get::<Tu>(posture_actor).copied(),
                app.world().get::<Stabilized>(downed_target).copied(),
                app.world().get::<LifeState>(downed_target).copied(),
                app.world().get::<LifeState>(downed_actor).copied(),
                // The move actor's Position + Tu, nested so the outer tuple stays within
                // the 12-element PartialEq/Debug tuple-arity ceiling.
                (
                    app.world().get::<Position>(move_actor).copied(),
                    app.world().get::<Tu>(move_actor).copied(),
                ),
            )
        };
        let before = snap(&app);

        // Run several updates writing NO messages.
        for _ in 0..3 {
            app.update();
        }

        assert_eq!(
            snap(&app),
            before,
            "with no `*Requested` emitted every dispatch system is inert — nothing mutates",
        );
    }

    // === AC7 — the dispatch systems co-schedule with the occupancy systems in
    // SimSystems::Simulate: the combined schedule builds + runs without an
    // ambiguity/access panic, and a fire that kills the target frees the grid slot
    // (sync_dead_gangers, in the same set, observes the LifeState change). ===

    #[test]
    fn dispatch_and_occupancy_co_schedule_and_a_kill_frees_the_slot() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        // BOTH plugins tag their systems into SimSystems::Simulate. OccupancyMaintenancePlugin
        // owns the set's configure_sets (E10.0); SimActsPlugin only `.in_set`s into it.
        app.add_plugins(OccupancyMaintenancePlugin);
        app.add_plugins(SimActsPlugin);
        insert_sim_resources(&mut app);

        // A shooter aiming at a LOW-HP in-line target so the volley downs/kills it.
        let mode = single_mode(0.2, 1);
        let shooter = spawn_shooter(app.world_mut(), 2, 5, mode, true);
        // Deliberately fragile target (1 HP, 1 Wound, paper armor) so the shot finishes
        // it — a relation (it dies), never a pinned damage number.
        let target = app
            .world_mut()
            .spawn(target_bundle(1, 1, worn_suit(0, 0, 1, 0)))
            .id();
        let target_at = CellLevel::new(Cell::new(8, 5), Level::new(0));
        // Place the target in the occupancy grid + give it a Position so the occupancy
        // move-sync writes its PrevSlot (the slot sync_dead_gangers later frees).
        app.world_mut()
            .entity_mut(target)
            .insert(Position::new(target_at));
        if let Some(mut grid) = app.world_mut().get_resource_mut::<OccupancyGrid>() {
            grid.set_occupant(target_at, Some(target));
            grid.set_occupant_band(target_at, Some(HeightBand::High));
        }
        // A LOW cover band at the target cell does not block a HIGH occupant; insert a
        // benign entry so the aim point reads a band (mirrors fire.rs in-line geometry).
        if let Some(mut cover) = app.world_mut().get_resource_mut::<CoverLedger>() {
            cover.insert(
                target_at,
                CoverEntry::seeded(
                    CoverHp::new(10),
                    HeightBand::High,
                    ArmorProtection::new(0),
                    ArmorHardness::new(0),
                ),
            );
        }

        // First update: sync_moved_gangers reacts to the target's Changed<Position>,
        // writing its PrevSlot and marking the occupancy slot. The combined schedule
        // builds + runs with no ambiguity/access panic (AC7's co-schedule check).
        app.update();
        let occupied_before = app
            .world()
            .get_resource::<OccupancyGrid>()
            .and_then(|g| g.occupant(&target_at));
        assert_eq!(
            occupied_before,
            Some(target),
            "after initial sync the target occupies its slot",
        );

        // Now FIRE — the dispatch mutates the target's LifeState (the kill), and in a
        // FOLLOWING deterministic update sync_dead_gangers (same set) observes the
        // Changed<LifeState> and frees the slot.
        app.world_mut().write_message(FireRequested::new(
            shooter,
            mode,
            Cell::new(8, 5),
            Level::new(0),
        ));
        app.update(); // dispatch_fire mutates LifeState this update
        app.update(); // sync_dead_gangers observes the change next update

        let life_after = app.world().get::<LifeState>(target).copied();
        let slot_after = app
            .world()
            .get_resource::<OccupancyGrid>()
            .and_then(|g| g.occupant(&target_at));
        // The fire downed or killed the target (a relation — not Alive).
        assert!(
            !matches!(life_after, Some(LifeState::Alive)),
            "the fire must have downed/killed the fragile target, got {life_after:?}",
        );
        // sync_dead_gangers (co-scheduled in the same set) freed the slot the dispatch's
        // LifeState change vacated — proving the two systems compose.
        assert_eq!(
            slot_after, None,
            "the occupancy slot the dispatched kill vacated must be freed by the \
             co-scheduled occupancy system",
        );
    }

    // === GTW-234 AC3 (dispatch path) — a VALID MoveRequested dispatch moves the actor
    // to the dest AND drops its Tu by EXACTLY the destination terrain's looked-up move
    // cost (a relation to the tuning leaf, never a pinned magnitude). ===

    /// Spawn a move-capable actor ([`Position`] / [`Tu`] / [`LifeState::Alive`]) at
    /// `(x, y, 0)`.
    fn spawn_move_actor(world: &mut World, x: i32, y: i32, tu: u8) -> Entity {
        world
            .spawn((
                Position::new(CellLevel::new(Cell::new(x, y), Level::new(0))),
                Tu::new(tu),
                LifeState::Alive,
            ))
            .id()
    }

    #[test]
    fn move_dispatch_steps_the_actor_and_spends_the_dest_terrain_cost() {
        let mut app = headless_app();
        let actor = spawn_move_actor(app.world_mut(), 10, 10, 100);
        let dest = CellLevel::new(Cell::new(11, 10), Level::new(0)); // Open, empty, in-bounds

        // The looked-up cost the dispatch will charge — read off the SAME resources the
        // dispatch reads (the dest's terrain × the move_costs table), a relation never a
        // pinned magnitude.
        let expected_cost = app.world().get_resource::<CombatTuning>().and_then(|t| {
            app.world()
                .get_resource::<OccupancyGrid>()
                .map(|g| *t.move_costs.cost(g.terrain(&dest)))
        });
        let tu_before = app.world().get::<Tu>(actor).map(|t| **t);

        app.world_mut()
            .write_message(MoveRequested::new(actor, dest));
        app.update();

        assert_eq!(
            app.world().get::<Position>(actor).copied(),
            Some(Position::new(dest)),
            "move dispatch must step the actor to the requested destination",
        );
        let tu_after = app.world().get::<Tu>(actor).map(|t| **t);
        assert!(
            matches!((tu_before, tu_after), (Some(b), Some(a)) if a < b),
            "a real move must strictly decrease Tu",
        );
        assert_eq!(
            tu_before.zip(tu_after).map(|(b, a)| b - a),
            expected_cost,
            "the Tu drop must equal exactly the destination terrain's looked-up move cost",
        );
    }

    // === GTW-234 AC7 — the move dispatch co-schedules with sync_moved_gangers in
    // SimSystems::Simulate: after a successful move + one update under the co-scheduled
    // harness, sync_moved_gangers sets the dest slot occupant and frees the source slot.
    // The combined schedule builds + runs with no ambiguity/access panic. ===

    #[test]
    fn move_dispatch_and_occupancy_co_schedule_fills_dest_and_frees_source() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        // BOTH plugins tag into SimSystems::Simulate; OccupancyMaintenancePlugin owns the
        // set's configure_sets, SimActsPlugin only `.in_set`s into it.
        app.add_plugins(OccupancyMaintenancePlugin);
        app.add_plugins(SimActsPlugin);
        insert_sim_resources(&mut app);

        let source = CellLevel::new(Cell::new(10, 10), Level::new(0));
        let dest = CellLevel::new(Cell::new(11, 10), Level::new(0)); // Open, empty, in-bounds
        let actor = spawn_move_actor(app.world_mut(), 10, 10, 100);

        // First update: sync_moved_gangers reacts to the actor's Changed<Position>
        // (initial placement), marking the source slot. The combined schedule builds +
        // runs with no ambiguity/access panic (AC7's co-schedule check).
        app.update();
        let source_occupied = app
            .world()
            .get_resource::<OccupancyGrid>()
            .and_then(|g| g.occupant(&source));
        assert_eq!(
            source_occupied,
            Some(actor),
            "after initial sync the actor occupies its source slot",
        );

        // Now MOVE — dispatch_move writes Position=dest this update; sync_moved_gangers
        // reacts to the Changed<Position> next update, marking the dest and freeing source.
        app.world_mut()
            .write_message(MoveRequested::new(actor, dest));
        app.update(); // dispatch_move writes Position=dest this update
        app.update(); // sync_moved_gangers reacts to Changed<Position> next update

        assert_eq!(
            app.world().get::<Position>(actor).copied(),
            Some(Position::new(dest)),
            "the dispatched move wrote Position=dest",
        );
        let grid = app.world().get_resource::<OccupancyGrid>();
        assert_eq!(
            grid.and_then(|g| g.occupant(&dest)),
            Some(actor),
            "sync_moved_gangers (co-scheduled) sets the dest slot occupant",
        );
        assert_eq!(
            grid.and_then(|g| g.occupant(&source)),
            None,
            "sync_moved_gangers (co-scheduled) frees the source slot",
        );
    }
}
