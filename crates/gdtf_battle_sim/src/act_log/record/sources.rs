//! The recorder INPUT bundles — one [`SystemParam`] per act family, so
//! [`record_acts`](super::record_acts) stays far under Bevy's 16-param system arity while
//! naming every source it reads (GTW-727 C9).
//!
//! Every source is an ALREADY-EMITTED output message or a read-only query. Nothing here
//! participates in combat resolution: no dispatcher gains a param, no RNG stream is drawn,
//! and no verdict is recomputed. The bundles are the `FireSignals` grouping precedent
//! (`acts/fire/signals.rs`) applied to the read side.

use bevy::{ecs::system::SystemParam, prelude::*};

use crate::{
    act_log::provenance::ActProvenance,
    acts::{
        FireDeclaration, InjuryInflicted, MeleeResolved, MeleeStruck, MoveRejected,
        MovementOccurred, ReloadResult, ThrowResolved,
    },
    armor_wear::ArmorBroken,
    battle::PlayerFaction,
    combatants::ganger::{Aiming, Facing, Hp, Position, Stance, Tu, Wounds},
    effects::{
        bleed::{BleedStarted, Bleeding},
        dot::{DotAfflicted, DotTicked},
        fields::{FieldAfflicted, FieldTicked},
        on_death::OnDeathOccurred,
    },
    falls::FallOccurred,
    ganger::{Faction, LifeState, Suppressed},
    inflicted_wound::InflictedWounds,
    injuries::InflictedInjuries,
    magazine::Magazine,
    occupancy_sync::CoverDestroyed,
    reaction::InterruptDeclared,
    shot_fired::ShotFired,
    suppression::SuppressionApplied,
    turn::{ActiveFaction, TurnStarted},
    weapon::WieldedBy,
};

/// The `(level, y, x)` sort key of a [`Position`] — the deterministic total order every
/// query-sourced recorder visits gangers in.
///
/// The SAME ordering convention the reaction trigger and the enemy brain already use for
/// their own snapshot passes, so a recorded pass order is a pure function of game state
/// rather than of archetype layout (`bevy-traps.md` #3). Kept local to the recorder family
/// rather than widening either existing private helper across a concern boundary.
pub(super) fn cell_order(position: &Position) -> (i32, i32, i32) {
    let key = ***position;
    (key.z, key.y, key.x)
}

/// The provenance reads — who the player is, whose turn it is, and each actor's faction.
///
/// The sim tags no request with its origin, so provenance is DERIVED here, once, from
/// state every recorder can see: an act by a player-faction ganger is
/// [`Commanded`](ActProvenance::Commanded); an act by any other faction on its own turn is
/// [`AiTurn`](ActProvenance::AiTurn); an act with no resolvable actor faction is
/// [`Clock`](ActProvenance::Clock). A reaction-fire interrupt overrides all three — the
/// fire recorder supplies it from [`InterruptDeclared`], which is the only source that
/// knows.
#[derive(SystemParam)]
pub struct ProvenanceSources<'w, 's> {
    /// The player's faction, absent outside a live battle (`bevy-traps.md` #1).
    pub(super) player:   Option<Res<'w, PlayerFaction>>,
    /// The faction currently taking its turn.
    pub(super) active:   Option<Res<'w, ActiveFaction>>,
    /// Each ganger's faction.
    pub(super) factions: Query<'w, 's, &'static Faction>,
}

impl ProvenanceSources<'_, '_> {
    /// The provenance of an ordinary (non-reaction) act by `actor`.
    pub(super) fn of(&self, actor: Entity) -> ActProvenance {
        let (Some(player), Ok(faction)) = (self.player.as_deref(), self.factions.get(actor)) else {
            // No resolvable player faction or no faction on the actor — the deed has no
            // commanding side, so it reads as an engine beat rather than a guess.
            return ActProvenance::Clock;
        };
        if *faction == **player {
            ActProvenance::Commanded
        } else {
            ActProvenance::AiTurn
        }
    }

    /// The provenance of a CLOCK beat that names an actor (a bleed / DOT / field tick).
    ///
    /// Always [`Clock`](ActProvenance::Clock): the ganger is the SUBJECT of the beat, not
    /// its author, so labelling it commanded or AI-driven would misattribute it.
    pub(super) const fn clock() -> ActProvenance {
        ActProvenance::Clock
    }

    /// Whether a turn is currently in progress at all — read so the turn recorder can
    /// stay silent outside one rather than guessing.
    pub(super) const fn turn_active(&self) -> bool {
        self.active.is_some()
    }
}

/// The TURN family's source — the turn-boundary signal.
#[derive(SystemParam)]
pub struct TurnSources<'w, 's> {
    /// Each turn advance the cycle engine emitted this tick.
    pub(super) turns: MessageReader<'w, 's, TurnStarted>,
}

/// The read-only posture columns the posture recorder visits, as a [`QueryData`] tuple so
/// the bundle's field stays under clippy's `type_complexity` gate.
///
/// [`QueryData`]: bevy::ecs::query::QueryData
type PostureColumns = (
    Entity,
    &'static Position,
    &'static Facing,
    &'static Stance,
    &'static Aiming,
    Option<&'static Suppressed>,
);

/// The POSTURE family's source — the live posture of every ganger, compared against the
/// log's prior-value map (C11).
#[derive(SystemParam)]
pub struct PostureSources<'w, 's> {
    /// Every ganger's position (the visit order) and posture components.
    pub(super) gangers: Query<'w, 's, PostureColumns>,
}

/// The MOVEMENT family's sources — the accepted step and the refused commit.
#[derive(SystemParam)]
pub struct MovementSources<'w, 's> {
    /// Each accepted walk step this tick.
    pub(super) steps:     MessageReader<'w, 's, MovementOccurred>,
    /// Each refused move commit this tick.
    pub(super) refusals:  MessageReader<'w, 's, MoveRejected>,
    /// Every ganger's settled position — the step's after-value AND the transition scan's
    /// source (a reposition with no step message: a fall, a shove, an emplacement entry).
    pub(super) positions: Query<'w, 's, (Entity, &'static Position)>,
}

/// The FIRE family's sources — the declaration, its rounds, and the reaction
/// discriminator.
#[derive(SystemParam)]
pub struct FireSources<'w, 's> {
    /// Each fire act declared this tick, carrying the rounds it emitted (C10).
    pub(super) declarations: MessageReader<'w, 's, FireDeclaration>,
    /// Every round resolved this tick, in volley order.
    pub(super) rounds:       MessageReader<'w, 's, ShotFired>,
    /// Each reaction interrupt declared this tick — the ONLY source that knows a shot was
    /// an interrupt and who it interrupted (C5).
    pub(super) interrupts:   MessageReader<'w, 's, InterruptDeclared>,
}

/// The CONSEQUENCE family's message sources — everything an act LEFT BEHIND.
#[derive(SystemParam)]
pub struct ConsequenceMessages<'w, 's> {
    /// Each resolved reload this tick.
    pub(super) reloads:       MessageReader<'w, 's, ReloadResult>,
    /// Each injury inflicted this tick.
    pub(super) injuries:      MessageReader<'w, 's, InjuryInflicted>,
    /// Each fall this tick.
    pub(super) falls:         MessageReader<'w, 's, FallOccurred>,
    /// Each connecting melee strike this tick.
    pub(super) strikes:       MessageReader<'w, 's, MeleeStruck>,
    /// Each terminal death / cover destruction this tick.
    pub(super) deaths:        MessageReader<'w, 's, OnDeathOccurred>,
    /// Each fresh suppression this tick.
    pub(super) suppressions:  MessageReader<'w, 's, SuppressionApplied>,
    /// Each armor piece that broke this tick.
    pub(super) armor_breaks:  MessageReader<'w, 's, ArmorBroken>,
    /// Each damage-over-time affliction that STARTED this tick.
    pub(super) dots:          MessageReader<'w, 's, DotAfflicted>,
    /// Each field exposure that STARTED this tick.
    pub(super) fields:        MessageReader<'w, 's, FieldAfflicted>,
    /// Each bleed-out span that STARTED this tick.
    pub(super) bleeds:        MessageReader<'w, 's, BleedStarted>,
    /// Each per-round bleed DRAIN this tick — the transient-FX fact, distinct from the
    /// once-per-span start above.
    pub(super) bleed_ticks:   MessageReader<'w, 's, Bleeding>,
    /// Each per-round damage-over-time DRAIN this tick — the transient-FX fact, distinct
    /// from the once-per-affliction start above.
    pub(super) dot_ticks:     MessageReader<'w, 's, DotTicked>,
    /// Each per-round FIELD drain this tick — the transient-FX fact, distinct from the
    /// once-per-exposure start above.
    pub(super) field_ticks:   MessageReader<'w, 's, FieldTicked>,
    /// Each piece of cover smashed this tick.
    pub(super) cover_smashed: MessageReader<'w, 's, CoverDestroyed>,
    /// Each close-combat strike that LANDED this tick (the strike-FX fact).
    pub(super) melee_landed:  MessageReader<'w, 's, MeleeResolved>,
    /// Each thrown grenade that LANDED this tick.
    pub(super) throw_landed:  MessageReader<'w, 's, ThrowResolved>,
}

/// The read-only vitals columns the vitals recorder visits, as a [`QueryData`] tuple so the
/// bundle's field stays under clippy's `type_complexity` gate.
///
/// [`QueryData`]: bevy::ecs::query::QueryData
type VitalsColumns = (
    Entity,
    &'static Position,
    &'static Tu,
    &'static Hp,
    &'static Wounds,
    Option<&'static InflictedWounds>,
    Option<&'static InflictedInjuries>,
);

/// The CONSEQUENCE family's query sources — the settled after-values no message owns.
#[derive(SystemParam)]
pub struct ConsequenceState<'w, 's> {
    /// Every ganger's position (the visit order) and vitals.
    pub(super) vitals:    Query<'w, 's, VitalsColumns>,
    /// Every wielded weapon's magazine and its wielder back-reference (the visit order is
    /// the WIELDER's position, so magazines order by the same game-state key as gangers).
    pub(super) magazines: Query<'w, 's, (Entity, &'static Magazine, &'static WieldedBy)>,
    /// Each wielder's position — the magazine visit-order key.
    pub(super) wielders:  Query<'w, 's, &'static Position>,
}

/// The LIFE family's source — every ganger's life state, compared against the log's
/// prior-value map so a genuine `from → to` transition is recorded (C11).
#[derive(SystemParam)]
pub struct LifeSources<'w, 's> {
    /// Every ganger's position (the visit order) and life state.
    pub(super) gangers: Query<'w, 's, (Entity, &'static Position, &'static LifeState)>,
}
