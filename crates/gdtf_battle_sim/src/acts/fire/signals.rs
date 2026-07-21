//! The fire boundary's output-signal types + writer bundle — the GTW-328
//! [`FireDeclaration`] combat-log signal and the [`FireSignals`] [`SystemParam`]
//! bundling every per-round output writer the dispatch emits on.

use bevy::{
    ecs::system::SystemParam,
    prelude::{Deref, Entity, Message, MessageWriter, Query},
};

use crate::{
    acts::injury::InjuryInflicted,
    ganger::Position,
    occupancy_sync::{CoverDestroyed, GroundAccrued, SlabDestroyed},
    shot_fired::ShotFired,
    weapon::ModeKind,
};

/// How many ROUNDS one fire act actually emitted — the length of the volley behind a
/// single [`FireDeclaration`].
///
/// A burst declares ONCE and fires several rounds, and the count is not knowable from the
/// request: the mode's nominal burst is CLAMPED to the magazine
/// ([`clamp_burst`](crate::magazine::clamp_burst)), so a half-empty weapon fires fewer
/// rounds than its mode names. Carrying the ACTUAL emitted count on the declaration is
/// what lets a consumer pair a declaration with exactly its own rounds even when one
/// shooter owns two declarations in the same tick (a reactor whose per-turn interrupt cap
/// is 2 or more).
///
/// A named newtype over the round count (`no-bare-types.md`): the inner is PRIVATE, read
/// through the derived [`Deref`].
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct RoundCount(u32);

impl RoundCount {
    /// No rounds — a declaration whose volley emitted nothing.
    pub const NONE: Self = Self(0);

    /// Build a round count from its total.
    #[must_use]
    pub const fn new(rounds: u32) -> Self {
        Self(rounds)
    }

    /// Build a round count from a volley's emitted-round length, saturating at
    /// [`u32::MAX`] (a volley can never approach it — the saturation exists so the
    /// conversion is total rather than a lossy cast).
    #[must_use]
    pub fn from_emitted(rounds: usize) -> Self {
        Self(u32::try_from(rounds).unwrap_or(u32::MAX))
    }
}

/// A **fire was declared** — the combat-log signal that `shooter` fired `mode` at
/// `target` (GTW-328), emitted ONCE per [`FireRequested`](crate::acts::request::FireRequested) that passes the firing-arc gate,
/// carrying the [`RoundCount`] its volley actually emitted.
///
/// The combat-text LOG event for a shot declaration ("`<name>` fired <Single/Burst/Full> at
/// `<target>`") — the user-facing announcement that a shot is being taken, distinct from the
/// per-round [`ShotFired`] outcome signal (a burst declares ONCE but fires multiple
/// rounds). It carries ONLY data the [`dispatch_fire`](super::dispatch::dispatch_fire) system already holds at fire time —
/// the [`shooter`](FireDeclaration::shooter) ref, the resolved [`target`](FireDeclaration::target)
/// occupant entity (if the aimed cell holds one, else `None`), and the
/// [`mode`](FireDeclaration::mode) [`ModeKind`] (read off the request's
/// [`FireModeSpec`](crate::weapon::FireModeSpec) kind) — plus, since GTW-727, the
/// [`rounds`](FireDeclaration::rounds) the volley emitted. It adds **no** fire-result logic,
/// performs **no** RNG draw, and re-resolves nothing — the determinism property is
/// untouched (`docs/combat/resolution.md` §"What's pure math vs sim").
///
/// **Emission point (GTW-727).** It used to be written BEFORE `fire()` rolled the volley;
/// it is now written immediately AFTER the per-round signals are emitted, because the
/// round count is not knowable until the volley exists (the mode's nominal burst is
/// clamped to the magazine). Nothing observes the declaration mid-volley — every consumer
/// drains it later in the same update — so the move is invisible to them; what it buys is
/// an exact declaration→rounds pairing.
///
/// A buffered Bevy [`Message`] (`bevy-traps.md` #4 — NOT the observer `Event`), written
/// with [`MessageWriter`] and read with [`MessageReader`](bevy::prelude::MessageReader), mirroring [`ShotFired`] /
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
    /// How many rounds this declaration's volley actually emitted (GTW-727) — the count of
    /// [`ShotFired`] messages that belong to THIS declaration, so a consumer can pair the
    /// two exactly even when one shooter declares twice in a tick.
    pub rounds:  RoundCount,
}

impl FireDeclaration {
    /// Build a fire-declaration signal for `shooter` firing `mode` at `target` (the
    /// resolved occupant entity, or `None` for an empty-cell shot), whose volley emitted
    /// `rounds` rounds.
    #[must_use]
    pub const fn new(
        shooter: Entity,
        target: Option<Entity>,
        mode: ModeKind,
        rounds: RoundCount,
    ) -> Self {
        Self {
            shooter,
            target,
            mode,
            rounds,
        }
    }
}

/// The output signal [`MessageWriter`]s [`dispatch_fire`](super::dispatch::dispatch_fire) emits on, bundled into one
/// [`SystemParam`] so the system's parameter list stays under clippy's argument-count gate
/// (the [`BattleGridsParam`](super::params::BattleGridsParam) grouping precedent above).
///
/// Grouping the cohesive output writers into one param keeps [`dispatch_fire`](super::dispatch::dispatch_fire) under the
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
    pub(super) shots:            MessageWriter<'w, ShotFired>,
    /// The per-REQUEST combat-log declaration (one per proceeding shot) — "<name> fired
    /// <mode> at <target>".
    pub(super) declarations:     MessageWriter<'w, FireDeclaration>,
    /// The per-ROUND cover-destroyed signal (GTW-364) — emitted for each round whose
    /// [`CoverVerdict`](crate::resolve_and_apply::CoverVerdict) records a destroyed
    /// cell, bridging the ledger's `deplete_cover` destruction into the buffered
    /// [`CoverDestroyed`] message that `sync_destroyed_cover` + `should_recompute_visibility`
    /// consume to free the cell + reopen LOS.
    pub(super) cover_destroyed:  MessageWriter<'w, CoverDestroyed>,
    /// The per-ROUND slab-destroyed signal (GTW-365) — emitted for each round whose
    /// [`SlabVerdict`](crate::resolve_and_apply::SlabVerdict) records a destroyed
    /// cell, bridging the ledger's `deplete_slab` destruction into the buffered
    /// [`SlabDestroyed`] message that `sync_destroyed_slab` (sets the slab
    /// [`SlabState::Destroyed`](crate::surface::SlabState) on the surface grid) +
    /// `should_recompute_visibility` consume to stop blocking rounds + reopen LOS
    /// through the hole. The slab mirror of `cover_destroyed`.
    pub(super) slab_destroyed:   MessageWriter<'w, SlabDestroyed>,
    /// The per-ROUND ground-accrued signal (GTW-366) — emitted for each round whose
    /// verdict is a [`GroundAccrual`](crate::resolve_and_apply::GroundAccrual),
    /// bridging the round's `weapon_damage` into the buffered
    /// [`GroundAccrued`] message that `sync_accrued_ground` accrues (monotonically) onto
    /// the [`SurfaceGrid`](crate::surface::SurfaceGrid)'s per-cell ground accumulator. The
    /// ground-accrual mirror of `slab_destroyed`: the ground is damaged-never-destroyed, so
    /// this signal carries an accrual, not a destruction (purely cosmetic — crater FX is a
    /// later ticket).
    pub(super) ground_accrued:   MessageWriter<'w, GroundAccrued>,
    /// The per-ROUND injury signal (GTW-438) — emitted for each round whose
    /// [`GangerVerdict::injury`](crate::resolve_and_apply::GangerVerdict::injury) is
    /// `Some`, bridging the in-fold injury roll into the buffered [`InjuryInflicted`] message
    /// [`apply_injury`](crate::acts::apply_injury) drains (and the presenter — GTW-439 —
    /// reads for the FCT / log flash). The injury-table mirror of the cover/slab/ground
    /// bridges: a structural hit destroys/accrues, a ganger wound INJURES.
    pub(super) injuries:         MessageWriter<'w, InjuryInflicted>,
    /// The per-ROUND DOT-applied signal (GTW-544) — emitted for each round whose
    /// [`GangerVerdict::dot_applied`](crate::resolve_and_apply::GangerVerdict::dot_applied) is
    /// `Some` (a penetrating hit from a DOT weapon), bridging the in-fold attach decision
    /// into the buffered [`DotApplied`](crate::effects::dot::DotApplied) message
    /// [`apply_dot`](crate::effects::dot::apply_dot) drains (attaching or REFRESHING the
    /// [`Dot`](crate::weapon::Dot) on the struck ganger). The DOT mirror of the injury
    /// bridge: a ganger wound that penetrated from a DOT weapon AFFLICTS.
    pub(super) dots:             MessageWriter<'w, crate::effects::dot::DotApplied>,
    /// The per-FIRE-ACT shove signal (GTW-525) — emitted ONCE when a `shove`-tagged weapon's
    /// shot CONNECTS with a ganger (the first connecting-ganger round of the volley). It
    /// writes an internal [`ShoveRequested`](crate::acts::request::ShoveRequested)
    /// (`ShoveSource::Weapon`) `dispatch_shove` drains the same frame (`dispatch_shove` is
    /// ordered `.after(dispatch_fire)`). A miss / a non-`shove` weapon writes nothing. Folded
    /// into this bundle so `dispatch_fire` stays under Bevy's 16-param limit.
    pub(super) shoves:           MessageWriter<'w, crate::acts::request::ShoveRequested>,
    /// The firing weapon's [`Shove`](crate::weapon::Shove) tag read (GTW-525) — read off the
    /// resolved ranged-weapon entity to decide whether a connecting shot auto-shoves. A
    /// read-only [`Query`] over the weapon entities, folded into this bundle (with the `shoves`
    /// writer) so `dispatch_fire` stays under the 16-param limit; disjoint from the ganger /
    /// magazine queries (a read on a different component set).
    pub(super) shove_tags:       Query<'w, 's, &'static crate::weapon::Shove>,
    /// The GTW-572 armor-broken signal — emitted per ROUND whose ganger verdict's §6 wear
    /// crossed a worn piece from protecting to broken
    /// ([`ArmorWearOutcome::Broke`](crate::armor_wear::ArmorWearOutcome::Broke), carried on
    /// [`AppliedDamage::wear`](crate::resolve_and_apply::AppliedDamage)). Bridges the in-fold
    /// crossing into the buffered [`ArmorBroken`](crate::armor_wear::ArmorBroken) message the
    /// presenter's spark flash / FCT tag and the combat log's armor-broken line drain —
    /// before GTW-572 the crossing rode the report but NOTHING emitted the message, so the
    /// armor-broken surfaces were dead in live play.
    pub(super) armor_breaks:     MessageWriter<'w, crate::armor_wear::ArmorBroken>,
    /// The GTW-547 terminal-death signal (`resolve_on_death` drains it) — emitted per ROUND
    /// (primary or splash) that KILLED a ganger (the ganger verdict's `life_after == Dead`),
    /// at the struck ganger's own cell (read via [`ganger_positions`](FireSignals::ganger_positions)).
    pub(super) deaths:           MessageWriter<'w, crate::effects::on_death::OnDeathOccurred>,
    /// The struck gangers' [`Position`] read (GTW-547) — the on-death signal needs the DEAD
    /// ganger's own `(cell, level)`, which a splash round's [`HitReport`](crate::resolve_and_apply::HitReport) does not carry
    /// (only the primary round has the outcome cell). A read-only [`Query`] over the ganger
    /// entities, disjoint from the weapon [`Query`]s, folded into this bundle.
    pub(super) ganger_positions: Query<'w, 's, &'static Position>,
}
