//! What happened in a single logged act.

use bevy::prelude::Entity;

use super::facts::{MagazineFacts, PoseFacts, PositionFacts, VitalsFacts};
use crate::{
    acts::{InjuryInflicted, MoveRejection, ReloadOutcome, RoundCount},
    armor::BodyPart,
    effects::fields::FieldDamage,
    falls::StoreysFallen,
    ganger::{Faction, LifeState},
    metric::{Cell, CellLevel, Level},
    resolve_hit::HpDamage,
    shot_fired::ShotFired,
    weapon::{DamageType, DotDamage, ModeKind},
};

/// Concrete event recorded in the act log.
#[derive(Debug, Clone, PartialEq)]
pub enum ActDeed {
    /// A faction's turn began.
    TurnBegan {
        /// Faction that is now active.
        now_active: Faction,
    },

    /// Facing / stance / aim / suppression changed.
    PostureChanged {
        /// New pose snapshot.
        pose: PoseFacts,
    },

    /// Single-cell step.
    Stepped {
        /// Cell left.
        from:     Cell,
        /// Cell entered.
        to:       Cell,
        /// Position after the step.
        position: PositionFacts,
    },

    /// Multi-cell move completed.
    MovedTo {
        /// Final position.
        position: PositionFacts,
    },

    /// Move was refused.
    MoveRefused {
        /// Why.
        reason: MoveRejection,
    },

    /// Fire was declared.
    Fired {
        /// Optional target entity.
        target: Option<Entity>,
        /// Fire mode.
        mode:   ModeKind,
        /// Rounds spent.
        rounds: RoundCount,
    },

    /// A single round resolved.
    RoundResolved {
        /// Shot outcome.
        shot: Box<ShotFired>,
    },

    /// Reload finished.
    Reloaded {
        /// Reload result.
        outcome: ReloadOutcome,
    },

    /// Magazine contents changed.
    MagazineChanged {
        /// New magazine snapshot.
        magazine: MagazineFacts,
    },

    /// Injury applied.
    Injured {
        /// Injury message payload.
        injury: Box<InjuryInflicted>,
    },

    /// Vitals snapshot changed.
    VitalsChanged {
        /// New vitals.
        vitals: VitalsFacts,
    },

    /// Fall completed.
    Fell {
        /// Level before the fall.
        from_level: Level,
        /// Landing level.
        to_level:   Level,
        /// Storeys fallen.
        storeys:    StoreysFallen,
    },

    /// HP damage applied to a target.
    Struck {
        /// Target entity.
        target:    Entity,
        /// HP removed.
        hp_damage: HpDamage,
    },

    /// Actor died.
    DiedAt {
        /// Death cell/level.
        at: CellLevel,
    },

    /// Suppression applied.
    Suppressed {
        /// Where the unit stood.
        at: CellLevel,
    },

    /// Armor piece broke.
    ArmorBroke {
        /// Body part.
        part: BodyPart,
    },

    /// Damage-over-time started.
    DotStarted {
        /// Damage per turn.
        per_turn: DotDamage,
    },

    /// Field effect started on a cell.
    FieldStarted {
        /// Cell of the field.
        at: CellLevel,
    },

    /// Bleed effect started.
    BleedStarted,

    /// Bleed tick occurred.
    Bled,

    /// DOT tick.
    DotTicked {
        /// Location.
        at:     CellLevel,
        /// Amount applied.
        amount: DotDamage,
    },

    /// Field tick.
    FieldTicked {
        /// Location.
        at:     CellLevel,
        /// Amount applied.
        amount: FieldDamage,
    },

    /// Cover was destroyed.
    CoverSmashed {
        /// Cover cell.
        at: CellLevel,
    },

    /// Melee hit landed.
    MeleeLanded {
        /// Hit location.
        at:     CellLevel,
        /// Damage type.
        damage: DamageType,
    },

    /// Thrown weapon/item landed.
    ThrowLanded {
        /// Impact location.
        at:     CellLevel,
        /// Damage type.
        damage: DamageType,
    },

    /// Life state transition.
    LifeChanged {
        /// Previous life state.
        from: LifeState,
        /// New life state.
        to:   LifeState,
        /// Position at change.
        at:   PositionFacts,
    },
}
