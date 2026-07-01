//! The **move** dispatch — the SINGLE writer that, on each buffered [`MoveRequested`]
//! commit, plans a reachable affordable route and (only then) starts the committed walk
//! (E7 · GTW-12f / GTW-354 / GTW-355; the original any-cell dispatch was E4 / GTW-234).
//!
//! ## What this slice adds (GTW-354)
//!
//! Before GTW-354 the dispatch let the original single-step `move_ganger` verb jump to ANY
//! single empty in-bounds cell — an any-empty-cell teleport (the destination need not be
//! adjacent or reachable). GTW-354 makes [`dispatch_move`] the single CONSTRAINED writer:
//! on each [`MoveRequested`] (the COMMIT — see below) it runs [`find_path`] from the
//! mover's cell to the requested [`CellLevel`] through the GTW-353 visibility-gated
//! [`PlanningView`] and:
//!
//! - **REJECTS** the move (a TYPED [`MoveRejected`], NO step) when no route exists
//!   ([`PathBlocked`] — the teleport is dead); and
//! - performs ONE up-front **full-route affordability** gate (`docs/combat/visibility.md`
//!   §48 — "the commit gates full-route affordability once, up front") against the
//!   [`Path::total`], rejecting (TYPED [`MoveRejected`], NO step) when the mover cannot
//!   afford the whole route.
//!
//! Only when a route exists AND is affordable does it accept: per GTW-355 it attaches a
//! [`WalkInProgress`] holding the planned route ahead (each cell's DESTINATION-terrain
//! per-step charge held verbatim), which [`advance_walk`](crate::move_acts::advance_walk)
//! then walks ONE cell per tick, plus the [`MovementOccurred`] log signal. No act logic is
//! reimplemented and the per-step charge is UNTOUCHED; the up-front gate here is a CHECK
//! against the planned total, NOT a second charge.
//!
//! ## Commit semantics (C2 — cross-ticket boundary)
//!
//! [`dispatch_move`] dispatches on [`MoveRequested`], which IS the commit (the 2nd /
//! commit click). It does NOT dispatch on a select / preview. The two-click INPUT (click-1
//! select+preview vs click-2 commit) is **GTW-356** and the preview DISPLAY is **GTW-358**
//! — NOT this slice. This dispatch treats every drained [`MoveRequested`] as a
//! commit-dispatch; it does no click-counting (that is GTW-356's input concern).
//!
//! It fetches the actor's components + reads the grids via Bevy queries / `Res`
//! (`bevy-traps.md` #7 — no `&mut World`); it only READS the grids + the squad fog.
//!
//! ## GTW-444 — "Hampered" movement-cost factor
//!
//! The dispatch reads the mover's
//! [`MovementCostFactor`](crate::injuries::MovementCostFactor) from its
//! [`InflictedInjuries`](crate::injuries::InflictedInjuries) ledger and passes it to
//! [`find_path`], which scales EVERY planar per-step floor cost by it. Because the
//! accepted [`WalkInProgress`] holds the planned [`Path::steps`](crate::pathfinder::Path::steps)
//! VERBATIM and [`advance_walk`](crate::move_acts::advance_walk) charges those steps, the
//! factor flows through to the actual per-step TU charge with NO second application — so a
//! Hampered unit's previewed path cost equals the TU it is charged (preview==charge, C3).
//! An uninjured mover passes the IDENTITY factor (`1.0`), leaving the cost unchanged.

use bevy::prelude::{Commands, Entity, Message, MessageReader, MessageWriter, Query, Res};

use crate::{
    acts::request::MoveRequested,
    battle::PlayerFaction,
    cover::CoverLedger,
    ganger::{Direction, Faction, Position, Suppressed, Tu},
    injuries::{InflictedInjuries, MovementCostFactor},
    metric::{Cell, CellLevel, Level},
    move_acts::WalkInProgress,
    occupancy::OccupancyGrid,
    pathfinder::{PlanningView, find_path},
    terrain::floor::FloorCostGrid,
    tu::can_spend_tu,
    tuning::CombatTuning,
    vertical::VerticalLinkGraph,
    visibility::{FactionRelation, OmniscientFog, SquadVisibility, move_fog},
};

/// Why a [`MoveRequested`] commit was **rejected** — the two no-step outcomes of the
/// GTW-354 route + affordability gate (C3).
///
/// A named domain enum (no-bare-types: a move's rejection reason is a domain value, not a
/// bare flag), mirroring the [`ReloadOutcome`](crate::acts::ReloadOutcome) shape. The
/// presenter / any reactive system classifies a [`MoveRejected`] by this variant. A move
/// that SUCCEEDS emits no [`MoveRejected`] (it emits [`MovementOccurred`] instead), so
/// there is no "accepted" variant here — and an actor missing a queried component is an
/// internal guard SKIP (no rejection signal at all, the `dispatch_*` precedent).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MoveRejection {
    /// No route exists from the mover's cell to the requested destination over the
    /// visibility-routable grid + links ([`find_path`] returned
    /// [`PathBlocked`](crate::pathfinder::PathBlocked)) — the destination is unreachable
    /// (it may be off-grid, walled off, behind an UNSEEN region, or blocked by a visible
    /// ganger). This is what kills the pre-GTW-354 any-empty-cell teleport.
    Unreachable,
    /// A route exists but the mover cannot afford its full-route [`Tu`] cost — the single
    /// up-front affordability gate (`docs/combat/visibility.md` §48) failed against the
    /// planned [`Path::total`](crate::pathfinder::Path::total). NO partial move: the mover
    /// stays put and spends nothing (GTW-355 owns the stepped walk; this is a CHECK, not a
    /// charge).
    Unaffordable,
    /// The mover is [`Suppressed`] and the chosen destination is ILLEGAL for a pinned unit
    /// (GTW-537, child GTW-41a of GTW-41; `docs/combat/combat.md` "Suppression … advanced
    /// combat effects"). A suppressed mover may ONLY step to a destination that is BOTH (a)
    /// STRICTLY FARTHER from the [`SuppressorCell`](crate::ganger::SuppressorCell) than its
    /// start cell (measured with the sim's Chebyshev ground-plane metric), AND (b) BEHIND
    /// COVER relative to the suppressor (the cell one step from the destination TOWARD the
    /// suppressor holds registered cover in the [`CoverLedger`]). A destination failing
    /// EITHER clause is a HARD REJECT (no clamp) — NO step. On a cover-sparse map this can
    /// pin the unit hard; that is the intended "pinned" feel (GTW-537 R1). An UNSUPPRESSED
    /// mover is never subject to this gate (identity).
    Suppressed,
}

/// A **move was rejected** — the typed no-step signal that `actor`'s commit could not be
/// dispatched, with the [`MoveRejection`] reason (GTW-354, C3).
///
/// A buffered Bevy [`Message`] (`bevy-traps.md` #4 — NOT the observer `Event`), mirroring
/// [`ReloadResult`](crate::acts::ReloadResult) / [`MovementOccurred`]. Emitted ONCE per
/// drained [`MoveRequested`] whose route gate fails — either no route
/// ([`MoveRejection::Unreachable`]) or an unaffordable route
/// ([`MoveRejection::Unaffordable`]). Neither [`Position`] nor [`Tu`] is touched on a
/// reject (a TOTAL no-op). The [`actor`](MoveRejected::actor) is a Bevy [`Entity`] handle
/// — framework plumbing, the only bare type the no-bare-types rule permits in a payload.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MoveRejected {
    /// The ganger whose move commit was rejected.
    pub actor:  Entity,
    /// Why the commit was rejected (no route, or an unaffordable route).
    pub reason: MoveRejection,
}

impl MoveRejected {
    /// Build a move-rejected signal for `actor` with the given [`MoveRejection`] reason.
    #[must_use]
    pub const fn new(actor: Entity, reason: MoveRejection) -> Self {
        Self { actor, reason }
    }
}

/// A **move occurred** — the combat-log signal that `actor` stepped from `from` to `to`
/// (GTW-328), emitted ONCE per accepted WALK STEP (GTW-355).
///
/// The combat-text LOG event for a move ("<name> moved <from> -> <to>") — the user-facing
/// announcement that a ganger changed cell. Since GTW-355 the committed walk
/// ([`advance_walk`](crate::move_acts::advance_walk)) emits ONE of these per DISCRETE step
/// it takes, so a multi-cell walk announces a step per cell entered; a rejected
/// (unreachable / unaffordable) commit emits a [`MoveRejected`] and no `MovementOccurred`,
/// and a bump-stopped / interrupted walk simply stops emitting them. The
/// [`from`](MovementOccurred::from) cell is the actor's pre-step ground cell and
/// [`to`](MovementOccurred::to) the cell it entered, so they are the actual pre/post ground
/// cells of THAT step. It adds **no** act logic and re-resolves nothing — pure exposure of
/// the step the walk already performed.
///
/// A buffered Bevy [`Message`] (`bevy-traps.md` #4 — NOT the observer `Event`), mirroring
/// [`crate::acts::ReloadResult`]. The [`actor`](MovementOccurred::actor) is a Bevy
/// [`Entity`] handle — framework plumbing, the only bare type the no-bare-types rule
/// permits in a payload; [`from`](MovementOccurred::from) / [`to`](MovementOccurred::to)
/// are the domain [`Cell`] newtype.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MovementOccurred {
    /// The ganger that stepped — resolved to a name by the combat-log presenter via
    /// `Query<&GangerName>`.
    pub actor: Entity,
    /// The ground [`Cell`] the actor stepped FROM (its pre-step cell).
    pub from:  Cell,
    /// The ground [`Cell`] the actor stepped TO (the cell it entered this step).
    pub to:    Cell,
}

impl MovementOccurred {
    /// Build a movement-occurred signal for `actor` stepping from `from` to `to`.
    #[must_use]
    pub const fn new(actor: Entity, from: Cell, to: Cell) -> Self {
        Self { actor, from, to }
    }
}

/// The faction relation of `occupant` **relative to** `mover_faction` — same gang is
/// [`FactionRelation::OwnSquad`], any other (or an occupant with no [`Faction`]) is
/// [`FactionRelation::Other`] (C1).
///
/// This is the [`PlanningView`] occupant→relation resolver, built per-commit from the
/// mover's faction and the live `&`[`Faction`] query. An occupant entity that is not a
/// faction ganger (an unexpected non-ganger occupant) maps to [`FactionRelation::Other`]
/// — the conservative classification: the gate then only blocks it when its cell is
/// squad-VISIBLE (it never silently blocks an unseen non-ganger). For the PLAYER mover
/// this is exactly "relative to the player squad", which is the relation
/// [`is_ganger_visible`](crate::visibility::is_ganger_visible) consumes.
fn relation_to(
    factions: &Query<&'static Faction>,
    mover_faction: Faction,
    occupant: Entity,
) -> FactionRelation {
    match factions.get(occupant) {
        Ok(faction) if *faction == mover_faction => FactionRelation::OwnSquad,
        _ => FactionRelation::Other,
    }
}

/// The ground-plane Chebyshev distance between two `(cell, level)` keys — `max(|dx|, |dy|)`
/// (GTW-537).
///
/// The sim's ESTABLISHED cell-distance metric (NOT a new one): the same `max(|dx|, |dy|)`
/// the AI's `chebyshev_xy` (`acts_runtime/ai/decide.rs`), the LOS engagement range gate, the
/// pathfinder heuristic, AND — decisively — the suppression producer's `within_radius`
/// (`acts_runtime/suppression/apply.rs`) all use, so "farther from the suppressor" agrees
/// with the disc suppression itself is measured on. The `z` storey is ignored (a ground
/// plane distance; suppression is a same-level effect this slice) — the suppressor anchor
/// and both the start and destination are on the mover's own storey by construction. A loop
/// magnitude (a comparison scalar, not a stored domain quantity), never a bare domain type.
fn chebyshev_xy(a: &CellLevel, b: &CellLevel) -> u32 {
    let dx = (a.x - b.x).unsigned_abs();
    let dy = (a.y - b.y).unsigned_abs();
    dx.max(dy)
}

/// Whether the cell one Moore-8 step from `dest` TOWARD `suppressor` holds registered cover
/// in the [`CoverLedger`] — the "ends behind cover relative to the suppressor" clause
/// (GTW-537).
///
/// Mirrors the GTW-526 auto-stance `cover_cell_toward` idiom
/// (`acts_runtime/suppression/stance.rs`): [`Direction::from_cells`] from `dest` toward the
/// `suppressor` cell picks the facing, [`Direction::cell_step`] the whole-cell delta, and the
/// stepped cell (kept on the destination's OWN storey — the cover a mover ducks behind is at
/// its level, not the suppressor's) is [`peek`](CoverLedger::peek)ed WITHOUT lazy seeding, so
/// a cell with no registered cover reads `None` = not behind cover. Returns `false` when the
/// destination and the suppressor share a ground cell (no direction — `from_cells` is `None`),
/// which is also not "farther", so such a destination is rejected on the distance clause too.
fn ends_behind_cover(dest: &CellLevel, suppressor: &CellLevel, cover: &CoverLedger) -> bool {
    let dest_cell = Cell::new(dest.x, dest.y);
    let suppressor_cell = Cell::new(suppressor.x, suppressor.y);
    let Some(dir) = Direction::from_cells(dest_cell, suppressor_cell) else {
        return false;
    };
    let step = dir.cell_step();
    let toward = Cell::new(dest_cell.x + step.x, dest_cell.y + step.y);
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "the destination's z is a storey index in 0..MAX_LEVELS (8) by construction, so \
                  the i32 -> u8 narrowing cannot truncate or sign-flip (the stance.rs pos_level \
                  precedent)"
    )]
    let level = Level::new(dest.z as u8);
    cover.peek(&CellLevel::new(toward, level)).is_some()
}

/// Whether a [`Suppressed`] mover may legally step from `start` to `dest` (GTW-537) — the
/// HARD-REJECT movement-strictness gate.
///
/// A pinned unit's chosen destination is LEGAL only when BOTH clauses hold (F-movement
/// strictness — a HARD reject of an illegal destination, never a clamp):
///
/// 1. `dest` is STRICTLY FARTHER from the [`SuppressorCell`](crate::ganger::SuppressorCell)
///    than `start` — [`chebyshev_xy`]`(dest, suppressor) > `[`chebyshev_xy`]`(start,
///    suppressor)` (the sim's existing Chebyshev metric, the same disc suppression is
///    measured on); and
/// 2. `dest` ENDS BEHIND COVER relative to the suppressor — [`ends_behind_cover`].
///
/// Failing EITHER clause is illegal (the caller rejects with [`MoveRejection::Suppressed`],
/// no step). On a cover-sparse map both clauses can be unsatisfiable, pinning the unit — the
/// intended "pinned" feel (GTW-537 R1), NOT softened.
fn suppressed_move_legal(
    start: &CellLevel,
    dest: &CellLevel,
    suppressor: &CellLevel,
    cover: &CoverLedger,
) -> bool {
    let farther = chebyshev_xy(dest, suppressor) > chebyshev_xy(start, suppressor);
    farther && ends_behind_cover(dest, suppressor, cover)
}

/// **Dispatch** buffered [`MoveRequested`] commits — for each, plan a reachable affordable
/// route and (only then) START the committed step-by-step walk (E7 · GTW-12f / GTW-12g;
/// GTW-354 added the route+affordability gate, GTW-355 made the accept start a walk
/// instead of jumping to the destination).
///
/// Per drained [`MoveRequested`] (the COMMIT — C2), the SINGLE constrained writer:
///
/// 1. fetches the mover's `(&`[`Position`]`, &`[`Tu`]`, &`[`Faction`]`)` (a message for an
///    actor missing any of these is SKIPPED — fail-closed, no panic, the `dispatch_*`
///    precedent; no [`MoveRejected`] for a guard skip); it reads, never mutates — the
///    [`Position`] / [`Tu`] mutation now lives in the walk;
/// 2. builds the GTW-353 [`PlanningView`] from the LIVE world — the [`SquadVisibility`]
///    resource plus an occupant→[`FactionRelation`] resolver ([`relation_to`]) closed over
///    the mover's faction and the `&`[`Faction`] query (C1) — and runs [`find_path`] from
///    the mover's cell to `request.dest` over the [`OccupancyGrid`], the
///    [`VerticalLinkGraph`], and the per-cell [`FloorCostGrid`] (GTW-396: the per-tile
///    floor move cost, replacing the coarse `move_costs` table; the flat
///    [`LinkTu`](crate::tuning::LinkTu) from [`CombatTuning`] still prices a link hop);
/// 3. on [`PathBlocked`](crate::pathfinder::PathBlocked) emits a TYPED
///    [`MoveRejected`]`(`[`MoveRejection::Unreachable`]`)` and starts NOTHING (this kills the
///    pre-GTW-354 any-empty-cell teleport — C1);
/// 4. GTW-537 — if the mover is [`Suppressed`], gates the CHOSEN destination through
///    [`suppressed_move_legal`] (strictly farther from the
///    [`SuppressorCell`](crate::ganger::SuppressorCell) by the Chebyshev metric AND behind
///    cover relative to the suppressor); an illegal destination is a HARD reject —
///    [`MoveRejected`]`(`[`MoveRejection::Suppressed`]`)`, NO step, NO clamp. An UNSUPPRESSED
///    mover skips this gate entirely (the identity path);
/// 5. performs ONE up-front full-route **affordability** gate
///    (`docs/combat/visibility.md` §48) — [`can_spend_tu`] against the [`Path::total`](crate::pathfinder::Path::total)
///    — emitting [`MoveRejected`]`(`[`MoveRejection::Unaffordable`]`)` and starting NOTHING
///    when the mover cannot afford the whole route (C3, NO walk); and
/// 6. ONLY on a reachable affordable route STARTS the walk (GTW-355, C6): it attaches a
///    [`WalkInProgress`](crate::move_acts::WalkInProgress) holding the route AHEAD
///    (`path.cells()[1..]`) and its planned per-step entry costs
///    ([`path.steps()`](crate::pathfinder::Path::steps), the §48 bit-identity source). The
///    landed [`advance_walk`](crate::move_acts::advance_walk) system then walks it ONE
///    discrete cell per tick — bump-stopping, charging per step, halting on reveal /
///    interrupt. A degenerate `start == goal` route (one cell) attaches NO walk.
///
/// The up-front gate is a CHECK, not a charge: the only TU spend is the walk's per-step
/// charge (`find_path` itself never charges TU — it plans and totals; §48). This system
/// does NOT write [`Position`] or [`Tu`] (the walk does), so the old single-frame jump is
/// gone — no teleport-to-destination remains.
///
/// Param-only (`Query` / `Res` / `MessageReader` / `MessageWriter` / `Commands`) — no
/// `&mut World` (`bevy-traps.md` #7). It READS the grids (`Res<OccupancyGrid>` /
/// `Res<VerticalLinkGraph>`) and the squad fog (`Res<SquadVisibility>`) for the route gate.
/// This system is ordered `.after(`the `occupancy_sync` maintenance chain`)` in the
/// [`SimSystems::Simulate`](crate::occupancy_sync::SimSystems::Simulate) set (C5 /
/// `bevy-traps.md` #3) so [`find_path`] plans over a grid whose occupant slots have already
/// settled this frame; `advance_walk` is ordered `.after(dispatch_move)` so the first walk
/// step lands the same frame as the accept.
#[expect(
    clippy::too_many_arguments,
    reason = "the constrained move dispatch genuinely needs the actor query + the disjoint \
              faction query + the disjoint suppression query (GTW-537 pinned-movement gate) + \
              the route-gate resources (grid / links / squad fog / tuning / floor costs / \
              cover ledger) + the GTW-70 faction-aware fog selection inputs (player faction + \
              omniscient fog) + the reject writer + Commands (to start the walk); bundling \
              them into an opaque SystemParam struct would hide the system's real reads (the \
              dispatch_fire BattleGridsParam precedent applies only when a bundle is reused \
              across systems)"
)]
pub fn dispatch_move(
    mut requests: MessageReader<MoveRequested>,
    actors: Query<(
        &'static Position,
        &'static Tu,
        &'static Faction,
        Option<&'static InflictedInjuries>,
    )>,
    factions: Query<&'static Faction>,
    // GTW-537: the mover's suppression state (if any). A DISJOINT read-only query (the actor
    // query never reads `Suppressed`), used to gate a PINNED mover's destination BEFORE the
    // walk starts. An UNSUPPRESSED mover has no `Suppressed` component, so `get` errs and the
    // gate is skipped — the identity path, byte-identical to the pre-GTW-537 dispatch.
    suppressed: Query<&'static Suppressed>,
    grid: Res<OccupancyGrid>,
    links: Res<VerticalLinkGraph>,
    squad: Res<SquadVisibility>,
    tuning: Res<CombatTuning>,
    floor_costs: Res<FloorCostGrid>,
    // GTW-537: the cover ledger — peeked at the cell one step from the destination toward the
    // suppressor to decide whether a pinned mover ends BEHIND COVER (read-only).
    cover: Res<CoverLedger>,
    // GTW-70: the faction-aware move-gate inputs. The player faction (to know if the mover
    // IS the player) and the AI's omniscient move fog, both battle-lifetime — taken as
    // `Option<Res<_>>` so a harness without them (no live battle) falls back to the player
    // fog, keeping player movement byte-identical to the pre-GTW-70 single-fog gate
    // (`bevy-traps.md` #1).
    player: Option<Res<PlayerFaction>>,
    omniscient: Option<Res<OmniscientFog>>,
    mut rejects: MessageWriter<MoveRejected>,
    mut commands: Commands,
) {
    for request in requests.read() {
        let Ok((position, tu, &mover_faction, injuries)) = actors.get(request.actor) else {
            // Fail-closed guard skip (no MoveRejected): a message for an actor missing a
            // queried component is silently dropped, the `dispatch_*` precedent.
            continue;
        };

        // GTW-444: the mover's per-ganger movement-cost factor (the "Hampered" slowdown),
        // read from its InflictedInjuries ledger — IDENTITY (1.0, no scaling) when the
        // ganger has no injury ledger or no MovementCostMul. The SAME factor scales
        // find_path's per-step costs AND (via the planned Path::steps the walk charges
        // verbatim) advance_walk's per-step TU charge, so preview == charge (C3).
        let factor = injuries.map_or(
            MovementCostFactor::IDENTITY,
            InflictedInjuries::movement_cost_factor,
        );

        // The mover's current cell — the route's start. (Routing is from where the mover
        // STANDS, so it keeps the storey.) `Position` derefs to the `CellLevel` `find_path`
        // expects (one deref — NOT the inner `IVec3`).
        let start: CellLevel = **position;

        // GTW-70: select the planning fog by MOVER FACTION through the shared `move_fog`
        // selector — the player moves on the squad fog (byte-identical to GTW-353), a
        // non-player mover (the AI) on the omniscient fog so it routes toward the player's
        // true cell without being gated by the *player's* fog (the reposition-soft-lock
        // fix). The enemy AI pre-checks against the IDENTICAL `move_fog` selection, so
        // planner and executor share ONE fog and cannot drift. A harness without the
        // player faction / omniscient fog (no live battle) falls back to the squad fog,
        // preserving the pre-GTW-70 gate exactly.
        let player_fog: &SquadVisibility = &squad;
        let planning_fog: &SquadVisibility = match (player.as_deref(), omniscient.as_deref()) {
            (Some(player), Some(omniscient)) => {
                move_fog(mover_faction, **player, player_fog, omniscient)
            }
            _ => player_fog,
        };

        // C1: build the GTW-353 visibility-gated planning view from the LIVE world — the
        // selected fog + the per-commit occupant→relation resolver (closed over the mover's
        // faction and the disjoint `&Faction` query). UNSEEN cells are non-routable; a
        // visible enemy / own-squad ganger blocks; EXPLORED stays routable.
        let planning = PlanningView::new(planning_fog, |occupant| {
            relation_to(&factions, mover_faction, occupant)
        });

        // C1: plan the route. No route → typed Unreachable reject, NO step (this kills the
        // pre-GTW-354 any-empty-cell teleport). GTW-396: floor_costs replaces the coarse
        // move_costs table as the per-step cost source inside find_path.
        let Ok(path) = find_path(
            start,
            request.dest,
            &grid,
            &links,
            &tuning,
            &floor_costs,
            factor,
            &planning,
        ) else {
            rejects.write(MoveRejected::new(request.actor, MoveRejection::Unreachable));
            continue;
        };

        // GTW-537: the SUPPRESSED-movement strictness gate (F-movement, HARD reject). A pinned
        // mover may step ONLY to a destination that is BOTH strictly farther from the
        // SuppressorCell (the sim's Chebyshev metric) AND ends behind cover relative to the
        // suppressor. An illegal destination is a HARD reject — no clamp, no partial step. An
        // UNSUPPRESSED mover has no `Suppressed` component (the `get` errs), so this gate is
        // skipped entirely: the identity path (byte-identical to the pre-GTW-537 dispatch). The
        // check is against `request.dest` (the CHOSEN destination), applied BEFORE the walk is
        // started, so a rejected suppressed mover never takes a step.
        if let Ok(suppressed) = suppressed.get(request.actor) {
            // `suppressed.from` is a `SuppressorCell`; deref through the newtype to the
            // `&CellLevel` the gate helpers take (the crate's `Deref`-newtype read path).
            if !suppressed_move_legal(&start, &request.dest, &suppressed.from, &cover) {
                rejects.write(MoveRejected::new(request.actor, MoveRejection::Suppressed));
                continue;
            }
        }

        // C3: ONE up-front full-route affordability gate (§48; GTW-354) — the planned
        // total vs the mover's Tu. Unaffordable → typed reject, NO walk started (the mover
        // stays put). This is a CHECK, not a charge — the per-step charges land in the walk
        // and always succeed (the up-front gate guarantees the whole route is payable, and
        // strictly turn-based play means nothing spends the mover's TU mid-walk).
        if !can_spend_tu(tu, path.total()) {
            rejects.write(MoveRejected::new(
                request.actor,
                MoveRejection::Unaffordable,
            ));
            continue;
        }

        // Reachable AND affordable → START the committed step-by-step walk (GTW-355, C6).
        // The accept no longer JUMPS the mover to the destination (the old single-frame
        // move_ganger teleport): it attaches a WalkInProgress holding the route AHEAD
        // (`cells[1..]`) and the planned per-step entry costs (`Path::steps()`, the §48
        // bit-identity source), which `advance_walk` then walks ONE discrete cell per tick
        // — bump-stopping on live obstacles, charging each step atomically with its
        // Position write, and halting on a reveal or a reaction interrupt. The per-step
        // Position writes are the discrete steps the presenter will tween (the visual tween
        // itself is a presenter follow-up, OUT OF SCOPE — GTW-355 C4). A degenerate
        // `start == goal` route (one cell, no steps) attaches no walk — there is nothing to
        // walk, and MovementOccurred is emitted only on a real step.
        let cells = path.cells();
        if cells.len() > 1 {
            commands
                .entity(request.actor)
                .insert(WalkInProgress::new(&cells[1..], path.steps()));
        }
    }
}
