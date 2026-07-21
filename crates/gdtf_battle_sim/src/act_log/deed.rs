//! [`ActDeed`] — WHAT happened, the closed vocabulary of the act log (GTW-727 C4).

use bevy::prelude::Entity;

use super::facts::{MagazineFacts, PoseFacts, PositionFacts, VitalsFacts};
use crate::{
    acts::{InjuryInflicted, MoveRejection, ReloadOutcome, RoundCount},
    armor::BodyPart,
    falls::StoreysFallen,
    ganger::{Faction, LifeState},
    metric::{Cell, CellLevel, Level},
    resolve_hit::HpDamage,
    shot_fired::ShotFired,
    weapon::{DamageType, DotDamage, ModeKind},
};

/// One recorded DEED — the closed enum of everything the act log can say happened.
///
/// ## Completeness bar
///
/// Exhaustive over the fifteen combat-log sources the presenter registers (its
/// `register_combat_log_forwarders`: the fourteen `CombatLogSource` messages plus the
/// bespoke turn-boundary forwarder), PLUS:
///
/// * [`LifeChanged`](Self::LifeChanged) — a genuine life-state TRANSITION, which no
///   message in the sim carries today;
/// * the three query-sourced state snapshots the drawn world needs
///   ([`PostureChanged`](Self::PostureChanged) / [`VitalsChanged`](Self::VitalsChanged) /
///   [`MagazineChanged`](Self::MagazineChanged));
/// * the four facts the presenter's transient-FX readers drive off, which are NOT combat-log
///   sources ([`Bled`](Self::Bled) / [`CoverSmashed`](Self::CoverSmashed) /
///   [`MeleeLanded`](Self::MeleeLanded) / [`ThrowLanded`](Self::ThrowLanded)). Without a
///   deed apiece those readers would have nothing to pace against, so their flashes would
///   keep firing at sim time while everything around them played at cursor time.
///
/// **Every mutating variant carries the AFTER value of what it changed** (C4). A consumer
/// APPLIES those values in log order; it never re-derives them from live world state,
/// which is precisely the defect the log exists to cure — a shot's damage must not appear
/// on screen before the shot's own bolt does. The multi-field after-values that no single
/// message owns (posture, vitals, magazine) each ride their own snapshot variant, recorded
/// from a change-detected transition; that keeps every damage path — shot, melee, fall,
/// bleed / DOT / field tick — carrying its settled numbers through ONE variant rather than
/// duplicating five fields onto nine message-sourced ones and still missing the tick paths.
///
/// ## Granularity
///
/// One entry per ROUND, not per volley: a burst records one [`Fired`](Self::Fired)
/// declaration carrying its [`RoundCount`], then that many
/// [`RoundResolved`](Self::RoundResolved) entries. That is what lets a consumer pace each
/// round of a burst individually, so the presenter needs no second staggering mechanism of
/// its own.
///
/// Derives [`PartialEq`] but NOT [`Eq`] / [`Hash`]: [`RoundResolved`](Self::RoundResolved)
/// boxes a [`ShotFired`], whose [`SimPos`](crate::metric::SimPos) /
/// [`ShotDir`](crate::sample_cone::ShotDir) hold `f32` — the same reasoning already
/// recorded on [`ShotFired`] itself (`shot_pipeline/shot_fired.rs`, "Derives `PartialEq`
/// (NOT `Eq`: the `SimPos` / `ShotDir` hold `f32` …)").
#[derive(Debug, Clone, PartialEq)]
pub enum ActDeed {
    /// A turn boundary — the named faction's turn just began.
    /// From [`TurnStarted`](crate::turn::TurnStarted).
    TurnBegan {
        /// The faction whose turn just started.
        now_active: Faction,
    },

    /// The actor's drawn POSTURE settled to a new combination (facing / stance / aim /
    /// suppression). Query-sourced from a transition, never a message.
    PostureChanged {
        /// The settled posture after the act.
        pose: PoseFacts,
    },

    /// The actor completed one walk STEP.
    /// From [`MovementOccurred`](crate::acts::MovementOccurred).
    Stepped {
        /// The ground cell stepped FROM.
        from:     Cell,
        /// The ground cell stepped TO.
        to:       Cell,
        /// The actor's settled `(cell, level)` after the step — the after-value, which the
        /// message's ground-cell pair alone cannot express across storeys.
        position: PositionFacts,
    },

    /// The actor's drawn POSITION settled somewhere it did not walk to — a fall, a shove,
    /// or any other one-shot reposition. Query-sourced from a transition, and recorded only
    /// when no [`Stepped`](Self::Stepped) already announced the move this tick, so an
    /// ordinary walk step costs ONE entry rather than two.
    MovedTo {
        /// The actor's settled `(cell, level)`.
        position: PositionFacts,
    },

    /// The actor's move commit was REFUSED (no route, or unaffordable) — a total no-op.
    /// From [`MoveRejected`](crate::acts::MoveRejected).
    MoveRefused {
        /// Why the commit was rejected.
        reason: MoveRejection,
    },

    /// The actor DECLARED a shot — one per fire act that passed the arc gate, emitted
    /// before the rounds resolve.
    /// From [`FireDeclaration`](crate::acts::FireDeclaration).
    Fired {
        /// The occupant standing in the aimed cell, if any.
        target: Option<Entity>,
        /// The declared fire mode's closed kind.
        mode:   ModeKind,
        /// How many rounds this declaration actually emitted — the count of
        /// [`RoundResolved`](Self::RoundResolved) entries that follow it for this actor.
        rounds: RoundCount,
    },

    /// ONE round of the actor's volley resolved — its trajectory and its verdict.
    /// From [`ShotFired`], boxed so this variant does not inflate every other one.
    RoundResolved {
        /// The round's geometry + already-computed hit report.
        shot: Box<ShotFired>,
    },

    /// The actor's RELOAD resolved.
    /// From [`ReloadResult`](crate::acts::ReloadResult).
    Reloaded {
        /// Which of the reload outcomes befell the actor.
        outcome: ReloadOutcome,
    },

    /// A weapon's MAGAZINE settled to a new load (a round left it, or a reload refilled
    /// it). The entry's actor is the WEAPON entity, not its wielder. Query-sourced from a
    /// transition.
    MagazineChanged {
        /// The weapon's settled magazine after the act.
        magazine: MagazineFacts,
    },

    /// The actor gained a named INJURY.
    ///
    /// Carries the whole [`InjuryInflicted`], boxed — the same shape
    /// [`RoundResolved`](Self::RoundResolved) uses, and for the same reason: the message
    /// holds an owned ledger entry plus three authored texts that no shorter summary could
    /// reconstruct, and a consumer showing the injury needs exactly what the sim rolled.
    Injured {
        /// The injury the actor gained.
        injury: Box<InjuryInflicted>,
    },

    /// The actor's VITALS settled to new values (TU / HP / wounds / inflicted wounds /
    /// injury ledger). Query-sourced from a transition, so every damage path — a shot's
    /// in-fold apply, a melee strike, a fall, a bleed / DOT / field tick — reports its
    /// settled numbers through this one variant.
    VitalsChanged {
        /// The actor's vitals after the act.
        vitals: VitalsFacts,
    },

    /// The actor FELL between storeys.
    /// From [`FallOccurred`](crate::falls::FallOccurred).
    Fell {
        /// The storey fallen FROM.
        from_level: Level,
        /// The storey landed ON.
        to_level:   Level,
        /// How many storeys were fallen.
        storeys:    StoreysFallen,
    },

    /// The actor STRUCK a target in close combat.
    /// From [`MeleeStruck`](crate::acts::MeleeStruck).
    Struck {
        /// The struck ganger.
        target:    Entity,
        /// The HP loss the connecting strike applied.
        hp_damage: HpDamage,
    },

    /// A terminal DEATH fired its on-death consequence at a cell. The actor is the dead
    /// ganger, or [`Entity::PLACEHOLDER`] for a destroyed piece of cover.
    /// From [`OnDeathOccurred`](crate::effects::on_death::OnDeathOccurred).
    DiedAt {
        /// Where the death / destruction happened.
        at: CellLevel,
    },

    /// The actor was freshly SUPPRESSED.
    /// From [`SuppressionApplied`](crate::suppression::SuppressionApplied).
    Suppressed {
        /// The suppressed ganger's cell.
        at: CellLevel,
    },

    /// The actor's worn ARMOR broke at a body part.
    /// From [`ArmorBroken`](crate::armor_wear::ArmorBroken).
    ArmorBroke {
        /// The body location whose worn piece crossed to broken.
        part: BodyPart,
    },

    /// The actor started bleeding a damage-over-time affliction.
    /// From [`DotAfflicted`](crate::effects::dot::DotAfflicted).
    DotStarted {
        /// The affliction's flat per-turn HP drain.
        per_turn: DotDamage,
    },

    /// The actor started taking damage from a placed FIELD it is standing in.
    /// From [`FieldAfflicted`](crate::effects::fields::FieldAfflicted).
    FieldStarted {
        /// The draining field's cell.
        at: CellLevel,
    },

    /// The actor's bleed-out span STARTED.
    /// From [`BleedStarted`](crate::effects::bleed::BleedStarted).
    BleedStarted,

    /// The actor BLED one round's worth — the per-round drain, distinct from the
    /// once-per-span start above (the log line fires once; the pop fires every round).
    /// From [`Bleeding`](crate::effects::bleed::Bleeding).
    Bled,

    /// A piece of COVER was smashed at a cell. The actor is the destroying shooter where
    /// one is known, else [`Entity::PLACEHOLDER`] (cover is not an entity).
    /// From [`CoverDestroyed`](crate::occupancy_sync::CoverDestroyed).
    CoverSmashed {
        /// The smashed cover's cell.
        at: CellLevel,
    },

    /// A close-combat strike LANDED at a cell — the strike FX's own fact, distinct from
    /// the number-bearing [`Struck`](Self::Struck) above.
    /// From [`MeleeResolved`](crate::acts::MeleeResolved).
    MeleeLanded {
        /// Where the strike landed.
        at:     CellLevel,
        /// The melee weapon's damage type — the FX role selector.
        damage: DamageType,
    },

    /// A thrown grenade LANDED at a cell.
    /// From [`ThrowResolved`](crate::acts::ThrowResolved).
    ThrowLanded {
        /// Where the grenade landed.
        at:     CellLevel,
        /// The grenade's damage type — the FX role selector.
        damage: DamageType,
    },

    /// The actor's LIFE STATE transitioned — the genuine `from → to` edge no sim message
    /// carries today (the wire's `Downed` / `Death` carve-out is sourced from exactly this
    /// change detection). Query-sourced from a transition.
    LifeChanged {
        /// The life state the actor left.
        from: LifeState,
        /// The life state the actor reached.
        to:   LifeState,
    },
}
