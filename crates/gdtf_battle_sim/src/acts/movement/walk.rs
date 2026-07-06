//! The **committed step-by-step walk** (E7 · GTW-12g / GTW-355): the persisted
//! [`WalkInProgress`] route state and the single [`advance_walk`] system that walks an
//! accepted route ONE cell per tick — bump-stopping on live obstacles, charging each
//! step atomically with its [`Position`] write, and halting the moment an enemy is
//! revealed or a reaction shot interrupts. See the module docs (`super`) for how this
//! replaces the old single-jump-to-destination move.

use bevy::{
    platform::collections::HashSet,
    prelude::{Commands, Entity, Message, MessageReader, MessageWriter, Query, Res},
};

use crate::{
    acts::MovementOccurred,
    ganger::{Faction, LifeState, Position, Tu},
    metric::CellLevel,
    occupancy::OccupancyGrid,
    tu::spend_tu,
    visibility::{FactionRelation, SquadVisibility, is_ganger_visible},
};

/// A **reaction shot was fired** at a walking mover — the typed interrupt the committed
/// walk stops on (GTW-355, C5(b)).
///
/// A buffered Bevy [`Message`] (`bevy-traps.md` #4 — NOT the observer `Event`): the
/// RECEIVING hook of the reaction-fire stop condition. [`advance_walk`] drains it and
/// halts the named `mover`'s walk at the cell it currently stands on (no further step,
/// no further charge — `charged = ground covered`).
///
/// **The PRODUCER is GTW-38-future (cross-ticket boundary).** Reaction fire / overwatch
/// does not exist anywhere in the sim yet (GTW-38 today is only the reaction-RATIO
/// denominator); NOTHING emits this message in production. This slice builds the
/// receiving hook ONLY — the future GTW-38 reaction-fire system will emit it when an
/// overwatching enemy flooses a shot at a mover crossing its sightline. The headless
/// test exercises it with a SYNTHETIC emit.
///
/// The [`mover`](ReactionShotFired::mover) is a Bevy [`Entity`] handle — framework
/// plumbing, the only bare type the no-bare-types rule permits in a payload.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ReactionShotFired {
    /// The walking ganger the reaction shot was fired at — its committed walk halts.
    pub mover: Entity,
}

impl ReactionShotFired {
    /// Build a reaction-shot interrupt aimed at the walking `mover`.
    #[must_use]
    pub const fn new(mover: Entity) -> Self {
        Self { mover }
    }
}

/// A **committed walk in progress** — the persisted route state an accepted move drives
/// cell-by-cell across ticks (GTW-355, C4).
///
/// A per-mover [`Component`](bevy::prelude::Component) the accept path of
/// [`dispatch_move`](crate::acts::dispatch_move) attaches once a route is planned and
/// gated affordable, and [`advance_walk`] consumes ONE step from per tick. It holds the
/// REMAINING route ahead of the mover — the cells it has yet to enter and each cell's
/// entry [`Tu`] cost (taken verbatim from the planned [`Path`](crate::pathfinder::Path),
/// so the per-step charge reproduces the planned total bit-for-bit, §48). It also pins
/// the reveal BASELINE (the set of enemy `(cell, level)`s the squad could already see
/// when the walk began), so a step that reveals a NEW enemy can be detected (C5(a)).
/// Each step's [`MovementOccurred`] `from`/`to` is the actual pre-/post-step ground cell
/// (read off the live [`Position`] at the step), so the per-step log is faithful.
///
/// The walk is NOT a single-frame loop: each [`advance_walk`] tick takes exactly ONE
/// step (a discrete [`Position`] write the presenter will tween — the visual tween
/// itself is a presenter concern, out of scope here), so a future reaction system or a
/// fresh obstacle can interrupt it MID-route. The component is REMOVED when the route is
/// exhausted, bump-stopped, or interrupted — its presence IS "this mover is walking".
///
/// Private fields + accessors (house style — a `WalkInProgress` is built only by the
/// move dispatch and advanced only by [`advance_walk`], never field-assembled from
/// outside the module). The remaining route is stored REVERSED (the next cell to enter
/// at the back) so a step is an O(1) `pop` rather than a front-shift.
#[derive(bevy::prelude::Component, Debug, Clone, PartialEq, Eq)]
pub struct WalkInProgress {
    /// The cells the mover has yet to ENTER, stored in REVERSE route order (the next
    /// cell to step onto is the LAST element), so advancing one step is an O(1) `pop`.
    remaining_cells: Vec<CellLevel>,
    /// The per-step ENTRY costs aligned to [`remaining_cells`](WalkInProgress::remaining_cells)
    /// (same reverse order, same length): popping a cell pops its matching cost. Each is
    /// the planned [`Path`](crate::pathfinder::Path) step cost, so the summed charge is
    /// the planned total bit-for-bit (§48).
    remaining_costs: Vec<Tu>,
    /// The reveal BASELINE: the squad-VISIBLE enemy `(cell, level)`s when the walk began.
    /// A step that makes the visible-enemy set a STRICT superset of this reveals a NEW
    /// enemy and halts the walk (C5(a)).
    ///
    /// Captured LAZILY on the walk's first [`advance_walk`] tick (`None` until then): the
    /// move dispatch can't read every ganger's [`Position`] without a `&mut Position` /
    /// `&Position` query conflict, and the first walk tick runs the SAME frame as the
    /// accept (ordered `.after(dispatch_move)`) with no `Position` write between them, so
    /// the lazily-captured baseline IS the fog at walk start.
    seen_enemies:    Option<HashSet<CellLevel>>,
}

impl WalkInProgress {
    /// Build a walk over a planned route's REMAINING cells + their per-step entry costs.
    ///
    /// `remaining_cells` is the route's `cells[1..]` (the cells AHEAD — the start cell is
    /// excluded, the mover already stands there) and `remaining_costs` the aligned
    /// `Path::steps()`; both are stored reversed internally for O(1) stepping. The caller
    /// (the move dispatch) passes them in forward order. The reveal baseline is captured
    /// on the first [`advance_walk`] tick (see
    /// [`seen_enemies`](WalkInProgress::seen_enemies)).
    #[must_use]
    pub fn new(remaining_cells: &[CellLevel], remaining_costs: &[Tu]) -> Self {
        let mut cells: Vec<CellLevel> = remaining_cells.to_vec();
        let mut costs: Vec<Tu> = remaining_costs.to_vec();
        cells.reverse();
        costs.reverse();
        Self {
            remaining_cells: cells,
            remaining_costs: costs,
            seen_enemies:    None,
        }
    }

    /// Whether the route is exhausted — no cells remain to enter.
    #[must_use]
    pub const fn is_complete(&self) -> bool {
        self.remaining_cells.is_empty()
    }

    /// The next cell the walk would step ONTO and its entry [`Tu`] cost, without
    /// consuming them — `None` once the route is exhausted.
    #[must_use]
    fn peek_next(&self) -> Option<(CellLevel, Tu)> {
        let cell = self.remaining_cells.last().copied()?;
        let cost = self.remaining_costs.last().copied()?;
        Some((cell, cost))
    }

    /// Consume the next step (the mover has now entered it).
    fn pop_next(&mut self) {
        self.remaining_cells.pop();
        self.remaining_costs.pop();
    }

    /// Whether the next visible-enemy set `current` reveals a NEW enemy versus the
    /// walk's baseline — capturing `current` AS the baseline on the first call (lazy
    /// init), in which case there is no reveal (the baseline is what was already seen).
    ///
    /// On the first tick the baseline is `None`, so `current` becomes the baseline and
    /// the call returns `false` (nothing is "new" relative to the moment the walk
    /// began). On every later tick a cell in `current` absent from the baseline is a
    /// freshly-revealed enemy → `true` (C5(a)).
    fn reveals_new_enemy(&mut self, current: &HashSet<CellLevel>) -> bool {
        match &self.seen_enemies {
            None => {
                self.seen_enemies = Some(current.clone());
                false
            }
            Some(baseline) => current.iter().any(|cell| !baseline.contains(cell)),
        }
    }
}

/// The squad-VISIBLE enemy `(cell, level)`s right now — the cells of every NON-mover
/// ganger whose cell is squad-VISIBLE, relative to `mover_faction` (GTW-355, C5(a)).
///
/// The reveal-detection seam reused at walk start (the baseline) and each tick (the
/// current set). It walks every ganger's `(Position, Faction)`, classifies its relation
/// to the mover ([`FactionRelation::Other`] for an enemy, [`FactionRelation::OwnSquad`]
/// for the mover's own gang), and keeps the enemy cells that
/// [`is_ganger_visible`](crate::visibility::is_ganger_visible) reports VISIBLE — the SAME
/// squad-fog read the presenter uses, so plan and render never disagree. Own-squad cells
/// are excluded (they are trivially visible and never the ambusher). The `mover` itself
/// is excluded.
///
/// Reads a SNAPSHOT of every ganger's `(Entity, cell, faction)` (taken once per tick to
/// avoid a `&mut Position` / `&Position` query conflict, `bevy-traps.md` #3) rather than
/// re-querying, so it can run alongside the mutable mover step.
fn visible_enemy_cells(
    mover: Entity,
    mover_faction: Faction,
    gangers: &[(Entity, CellLevel, Faction)],
    squad: &SquadVisibility,
) -> HashSet<CellLevel> {
    gangers
        .iter()
        .filter(|(entity, ..)| *entity != mover)
        .filter_map(|(_, cell, faction)| {
            let relation = if *faction == mover_faction {
                FactionRelation::OwnSquad
            } else {
                FactionRelation::Other
            };
            // Only an ENEMY whose cell is squad-VISIBLE is a reveal candidate.
            if relation == FactionRelation::Other && is_ganger_visible(squad, cell, relation) {
                Some(*cell)
            } else {
                None
            }
        })
        .collect()
}

/// **Advance** every [`WalkInProgress`] by ONE step this tick — the committed-walk engine
/// (E7 · GTW-12g / GTW-355).
///
/// For each ganger carrying a [`WalkInProgress`], in priority order:
///
/// 1. **Stop on a reaction shot (C5(b))** — if a [`ReactionShotFired`] named this mover
///    this tick, halt: remove the walk, take no step, charge nothing
///    (`charged = ground covered`). The PRODUCER is GTW-38-future; today only a synthetic
///    emit triggers it.
/// 2. **Stop on a reveal (C5(a))** — recompute the squad-VISIBLE enemy cells; if a NEW
///    enemy is visible that was NOT in the walk's baseline (a strict superset), halt:
///    remove the walk, take no step. The previous tick's step wrote a new [`Position`],
///    which re-ran [`recompute_visibility`](crate::visibility::recompute_visibility)
///    (ordered before this system on the next tick) — so this read sees the fog as of
///    where the mover STOPPED (the §44 ambush invariant). The reveal IS detected from the
///    [`SquadVisibility`] output, never re-deriving FOV.
/// 3. **Bump-stop on a live obstacle (C1; GTW-501 D2)** — peek the next cell; if the LIVE
///    [`OccupancyGrid`] now reports it
///    [`is_path_blocked`](crate::occupancy::OccupancyGrid::is_path_blocked) — the SAME
///    tag-derived path-blocking surface the PLANNER reads, NOT the kind-based
///    [`is_blocked`](crate::occupancy::OccupancyGrid::is_blocked) (so planner and executor
///    agree on a cell that path-blocks via tag ONLY, e.g. a `BlocksPathfinding` marker added
///    onto the route mid-walk) — OR occupied (a ganger or blocking scatter that arrived since
///    the route was planned — occupancy is re-synced every tick), halt at the last free step:
///    remove the walk, take NO step onto the obstacle (no teleport, no co-location).
/// 4. **Take ONE step (C2 / C4)** — otherwise write the mover's [`Position`] to the next
///    cell (a single discrete write the presenter tweens), charge that step's planned
///    entry cost atomically via [`spend_tu`], announce a [`MovementOccurred`] from the
///    walk's origin to the entered cell, and consume the step. If the route is now
///    exhausted, remove the walk (the destination is reached).
///
/// Exactly ONE step is taken per mover per tick, so the walk advances cell-by-cell ACROSS
/// ticks and any interrupt above lands at a cell boundary (C4). The up-front affordability
/// gate already passed in [`dispatch_move`](crate::acts::dispatch_move), so a step's charge
/// always succeeds (strictly turn-based — nothing else spends the mover's TU mid-walk),
/// and the per-step costs sum to the planned total bit-for-bit when uninterrupted (§48).
///
/// Param-only (`Query` / `Res` / `MessageReader` / `MessageWriter` / `Commands`) — no
/// `&mut World` (`bevy-traps.md` #7). It is ordered `.after` the `occupancy_sync` grid
/// maintenance (so the bump-stop reads a settled grid) and BEFORE
/// [`recompute_visibility`](crate::visibility::recompute_visibility) (so a step's reveal
/// is computed before the NEXT tick reads it); see
/// [`SimActsPlugin`](crate::acts::SimActsPlugin).
#[expect(
    clippy::type_complexity,
    reason = "the committed walk genuinely needs a ParamSet of the MUTABLE mover query \
              (step + charge + route state) and the READ-ONLY ganger snapshot query \
              (reveal detection) — they overlap on Position (one `&mut`, one `&`), so a \
              ParamSet time-multiplexes them to avoid an alias conflict (bevy-traps.md #3, \
              the dispatch_fire precedent); Bevy's IntoSystem inference rejects a \
              lifetime-generic type alias for the ParamSet, so it stays inline"
)]
pub fn advance_walk(
    mut world: bevy::ecs::system::ParamSet<(
        Query<(
            Entity,
            &'static mut Position,
            &'static mut Tu,
            &'static LifeState,
            &'static Faction,
            &'static mut WalkInProgress,
        )>,
        Query<(Entity, &'static Position, &'static Faction)>,
    )>,
    grid: Res<OccupancyGrid>,
    squad: Res<SquadVisibility>,
    mut reactions: MessageReader<ReactionShotFired>,
    mut moves: MessageWriter<MovementOccurred>,
    mut commands: Commands,
) {
    // Drain the reaction interrupts ONCE this tick — a set of movers whose walk must stop
    // (the GTW-38-future producer emits these; today only a synthetic test emit does).
    let interrupted: HashSet<Entity> = reactions.read().map(|shot| shot.mover).collect();

    // Snapshot every ganger's (Entity, cell, faction) for reveal detection, taken from
    // the read-only ganger query (p1) BEFORE the mutable mover step (p0) — a ParamSet so
    // the `&mut Position` mover step and the `&Position` reveal read never alias
    // (bevy-traps.md #3 — the dispatch_fire ParamSet precedent).
    let snapshot: Vec<(Entity, CellLevel, Faction)> = world
        .p1()
        .iter()
        .map(|(entity, position, faction)| (entity, **position, *faction))
        .collect();

    for (mover, mut position, mut tu, &life, &faction, mut walk) in &mut world.p0() {
        // A dead/downed mover never walks — fail-closed, drop the stale walk.
        if !matches!(life, LifeState::Alive) {
            commands.entity(mover).remove::<WalkInProgress>();
            continue;
        }

        // C5(b): a reaction shot at this mover halts the walk at its current cell.
        if interrupted.contains(&mover) {
            commands.entity(mover).remove::<WalkInProgress>();
            continue;
        }

        // C5(a): a NEW squad-visible enemy (a cell absent from the walk's baseline) halts
        // the walk — the reveal is read off the SquadVisibility the previous step's
        // Position write already refreshed (§44 ambush invariant; FOV is NOT re-derived).
        // The first tick captures the current set AS the baseline (no reveal yet).
        let visible_now = visible_enemy_cells(mover, faction, &snapshot, &squad);
        if walk.reveals_new_enemy(&visible_now) {
            commands.entity(mover).remove::<WalkInProgress>();
            continue;
        }

        // Peek the next cell; an exhausted route (no cells left) is a finished walk.
        let Some((next, cost)) = walk.peek_next() else {
            commands.entity(mover).remove::<WalkInProgress>();
            continue;
        };

        // C1 (bump-stop): re-check LIVE model truth before ENTERING the next cell. A cell
        // that path-blocks now there is `is_path_blocked`; a ganger or blocking scatter that
        // arrived since planning occupies the slot. Either halts at the last free step —
        // NO step onto the obstacle, NO teleport, NO co-location.
        //
        // GTW-501 D2: the bump-stop is a RUNTIME movement-impassability gate, so it MUST read
        // the SAME tag-derived path-blocking surface the planner reads
        // ([`is_path_blocked`](OccupancyGrid::is_path_blocked) — `pathable_neighbors` /
        // `find_path` / `reachable_within`), NEVER the kind-based
        // [`is_blocked`](OccupancyGrid::is_blocked) (that stays vision-only, C4). Otherwise a
        // cell that path-blocks via tag ONLY (an explicitly-tagged Slab per D2, or a
        // `BlocksPathfinding` marker added at runtime onto an in-progress walk's already-planned
        // route per C3) — where `is_path_blocked == true` but `is_blocked == false` — would slip
        // the bump-stop and the mover would walk THROUGH a cell the planner treats as
        // impassable. Reading `is_path_blocked` keeps planner and executor in lock-step. The
        // zero-regression Wall/Cover case is unaffected (both queries agree there, C5).
        if grid.is_path_blocked(&next) || grid.occupant(&next).is_some() {
            commands.entity(mover).remove::<WalkInProgress>();
            continue;
        }

        // C2 / C4: take ONE discrete step — write Position, charge the planned step cost
        // atomically, announce the move from the pre-step ground cell to the entered one,
        // consume the step. The `from` is the live pre-write Position (each step logs its
        // own real pre/post cells, the GTW-328 MovementOccurred contract).
        // The canonical CellLevel::cell accessor (GTW-565) for the movement log's
        // ground cells.
        let from = position.cell();
        *position = Position::new(next);
        spend_tu(&mut tu, cost);
        moves.write(MovementOccurred::new(mover, from, next.cell()));
        walk.pop_next();

        // Route exhausted → the destination is reached; remove the walk.
        if walk.is_complete() {
            commands.entity(mover).remove::<WalkInProgress>();
        }
    }
}
