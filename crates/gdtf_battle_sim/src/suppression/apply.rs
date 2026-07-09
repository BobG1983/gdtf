//! The **suppression producer** [`apply_suppression`] + its presenter-facing
//! [`SuppressionApplied`] signal (GTW-526 C2, child of GTW-41).
//!
//! On every enemy-of-the-target [`FireRequested`](crate::acts::FireRequested), every
//! OPPOSING-faction ganger within
//! [`SuppressionRadius`](crate::tuning::SuppressionRadius) of the shot's target cell is
//! marked [`Suppressed`], anchored to the SHOOTER's own
//! [`Position`](crate::ganger::Position) (the origin of the fire — the directional
//! anchor, NOT the aim cell). A FRESH application (the unit was not already suppressed)
//! emits a [`SuppressionApplied`] for the presenter's suppression FCT; a re-application
//! of an already-suppressed unit is an IDEMPOTENT refresh — its
//! [`SuppressorCell`](crate::ganger::SuppressorCell) updates to the newest shooter, but
//! no second signal fires and the suppression does not stack.

use bevy::prelude::{Commands, Entity, Message, MessageReader, MessageWriter, Query, Res, With};

use crate::{
    acts::FireRequested,
    fire::{MeleeQuery, MountedQuery, WieldsQuery},
    ganger::{Faction, Position, Suppressed, SuppressorCell},
    metric::{Cell, CellLevel, Level},
    tuning::CombatTuning,
    weapon::{Silenced, shooter_weapon_silenced},
};

/// A ganger was **freshly suppressed** at `at` — the presenter-facing signal a
/// suppression FCT (floating combat text) keys off (GTW-526 C2).
///
/// A buffered [`Message`] (`bevy-traps.md` #4 — NOT the observer `Event`) emitted by
/// [`apply_suppression`] exactly ONCE per ganger that transitions from un-suppressed to
/// suppressed this tick. An IDEMPOTENT refresh of an already-suppressed unit emits
/// nothing (no double-pop). It mirrors the other presenter-facing sim signals
/// ([`MeleeResolved`](crate::acts::MeleeResolved) /
/// [`ShotFired`](crate::shot_fired::ShotFired)): it carries ONLY what the view needs —
/// the suppressed ganger (GTW-572: resolved to a name for the suppression log line) and
/// its cell — never combat math. The presenter reads it through a
/// [`MessageReader`](bevy::prelude::MessageReader) (the one-way sim → presenter dep).
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SuppressionApplied {
    /// The freshly-suppressed ganger — resolved to a display name at the presenter
    /// boundary for the GTW-572 suppression log line. A Bevy [`Entity`] handle (framework
    /// plumbing, the only bare type the no-bare-types rule permits in a payload).
    pub ganger: Entity,
    /// The `(cell, level)` of the newly-suppressed ganger — where the suppression FCT
    /// draws. A [`CellLevel`] newtype, never a bare `IVec3`.
    pub at:     CellLevel,
}

impl SuppressionApplied {
    /// Build a suppression-applied signal for `ganger`, freshly suppressed at `at`.
    #[must_use]
    pub const fn new(ganger: Entity, at: CellLevel) -> Self {
        Self { ganger, at }
    }
}

/// Whether the ganger at `(cell, level)` is within the suppression `radius` of the
/// shot's `(target_cell, target_level)` — a Chebyshev disc on the ground plane, gated to
/// the SAME storey.
///
/// Radius `0` matches ONLY the directly-targeted occupant cell (Chebyshev `0`); a larger
/// radius pins a wider Moore neighbourhood. A ganger on a DIFFERENT storey is never
/// suppressed by this shot (suppression is a same-level effect this slice).
fn within_radius(
    cell: Cell,
    level: Level,
    target_cell: Cell,
    target_level: Level,
    radius: u8,
) -> bool {
    if *level != *target_level {
        return false;
    }
    let dx = (cell.x - target_cell.x).unsigned_abs();
    let dy = (cell.y - target_cell.y).unsigned_abs();
    dx.max(dy) <= u32::from(radius)
}

/// The **suppression producer** — mark every OPPOSING-faction ganger within the tuning
/// [`SuppressionRadius`](crate::tuning::SuppressionRadius) of an enemy shot's target as
/// [`Suppressed`], anchored to the shooter's origin (GTW-526 C2).
///
/// ## What it reads
///
/// Every [`FireRequested`](crate::acts::FireRequested) this tick carries the `shooter`
/// entity + the aimed `(target_cell, target_level)`. The shooter's own
/// [`Position`](crate::ganger::Position) + [`Faction`](crate::ganger::Faction) resolve
/// the fire's ORIGIN (the [`SuppressorCell`] anchor) and its side; every OTHER ganger's
/// `(Entity, &Position, &Faction, Option<&Suppressed>)` is the candidate set.
///
/// ## What it does (per fire request)
///
/// For each ganger of a DIFFERENT faction than the shooter that lies
/// `within_radius` of the target:
///
/// - if it was NOT already [`Suppressed`], insert `Suppressed { from: <shooter cell> }`
///   via [`Commands`](bevy::prelude::Commands) AND emit a [`SuppressionApplied`] (the
///   fresh-application FCT signal); OR
/// - if it WAS already suppressed, re-insert `Suppressed` with the NEWEST shooter's cell
///   (an idempotent REFRESH — the anchor updates, but no second signal fires and the
///   suppression does not stack).
///
/// A ganger that both re-fires and freshly suppresses in the same tick is handled per
/// request in order; the last write wins for the anchor, and a fresh signal fires only
/// on the first (un-suppressed → suppressed) transition — tracked in a local
/// already-suppressed set so a second request the same tick sees the just-applied state.
///
/// Symmetric: the faction split is purely `faction != shooter_faction`, so a PLAYER shot
/// suppresses enemies and an ENEMY shot suppresses players (BOTH factions can be
/// suppressed).
///
/// GTW-542 / GTW-674: a SILENCED shooter's shot produces NO suppression. Before marking
/// anyone, the producer resolves the shooter's FIRING weapon through the shared preference
/// rule ([`Wields::firing_weapon`](crate::weapon::Wields::firing_weapon), mounted-first via
/// the `mounted` probe, melee-excluded via the `melee` probe — the `dispatch_fire`
/// resolution) and SKIPS the whole fire request if that weapon carries the [`Silenced`] tag
/// (a `silenced` probe). So a MOUNTED shooter's suppression tracks the mount it fires (not
/// its carried gun), and a suppressor removes the shot's NOISE footprint entirely,
/// faction-blind.
///
/// Param-only (`MessageReader` / `Query` / `Res` / `Commands` / `MessageWriter`) — no
/// `&mut World` (`bevy-traps.md` #7). Reads [`CombatTuning`] as `Option<Res>` so a
/// harness without it fails closed (no suppression) rather than panicking — mirroring the
/// `dispatch_fire` / `dispatch_open_door` `Option<Res>` precedent (`bevy-traps.md` #1).
#[expect(
    clippy::too_many_arguments,
    reason = "the producer reads the FireRequested buffer, the shooter position/faction \
              query, the candidate-ganger query, the GTW-542/674 silenced-weapon resolution (the \
              wield + mounted-probe + melee-probe + Silenced-marker queries), the \
              Option<CombatTuning>, and the Commands + SuppressionApplied writer; each is a \
              distinct Bevy SystemParam (mirroring dispatch_fire's own carve-out) — bundling \
              would only hide the reads"
)]
pub fn apply_suppression(
    mut fires: MessageReader<FireRequested>,
    positions: Query<(&Position, &Faction)>,
    gangers: Query<(Entity, &Position, &Faction, Option<&Suppressed>)>,
    // GTW-542 / GTW-674: resolve the shooter's FIRING weapon (mounted-first) to gate on its
    // `Silenced` tag.
    wields: WieldsQuery,
    mounted: MountedQuery,
    melee: MeleeQuery,
    silenced: Query<(), With<Silenced>>,
    tuning: Option<Res<CombatTuning>>,
    mut commands: Commands,
    mut applied: MessageWriter<SuppressionApplied>,
) {
    // Fail closed on the tuning's absence (a lean harness) — no radius, no suppression.
    let Some(tuning) = tuning else {
        return;
    };
    let radius = *tuning.reaction.suppression_radius;

    // Track units suppressed THIS tick so a second fire request the same tick treats an
    // already-just-suppressed unit as a REFRESH (no double signal) — the live Query's
    // Option<&Suppressed> only reflects a Commands-inserted component NEXT tick.
    let mut suppressed_this_tick: bevy::platform::collections::HashSet<Entity> =
        bevy::platform::collections::HashSet::default();

    for fire in fires.read() {
        // GTW-542 / GTW-674: a SILENCED shot makes NO noise — skip the whole request before
        // marking anyone. Resolve the shooter's FIRING weapon (mounted-first, excluding its
        // melee weapon) and skip if it carries the Silenced tag. A shooter with no resolvable
        // firing weapon is treated as un-silenced (fall through — the shot came from somewhere
        // loud).
        if shooter_weapon_silenced(fire.shooter, &wields, &mounted, &melee, &silenced) {
            continue;
        }
        // Resolve the shooter's origin + side; a shooter missing from the query (an
        // unexpected non-ganger) cannot suppress — skip it (fail closed).
        let Ok((shooter_position, shooter_faction)) = positions.get(fire.shooter) else {
            continue;
        };
        let suppressor = SuppressorCell::new(**shooter_position);

        for (entity, position, faction, already) in &gangers {
            // OPPOSING faction only (a shooter never suppresses its own gang), and never
            // the shooter itself.
            if *faction == *shooter_faction || entity == fire.shooter {
                continue;
            }
            // The canonical CellLevel accessors through Position's deref (GTW-565).
            if !within_radius(
                position.cell(),
                position.level(),
                fire.target_cell,
                fire.target_level,
                radius,
            ) {
                continue;
            }

            // A FRESH application iff the unit is neither already-Suppressed in the world
            // NOR suppressed earlier this tick. Otherwise an idempotent REFRESH.
            let is_fresh = already.is_none() && !suppressed_this_tick.contains(&entity);
            commands.entity(entity).insert(Suppressed::new(suppressor));
            if is_fresh {
                applied.write(SuppressionApplied::new(entity, **position));
                suppressed_this_tick.insert(entity);
            }
        }
    }
}
