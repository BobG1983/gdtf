mod blast;
mod fall;
mod fct;
mod flash;
mod impact;
mod melee;
mod projectile;
mod readers;
mod registrar;
mod roles;
mod tuning;

#[cfg(test)]
mod test;

pub use blast::read_throw_resolved;
pub use fall::read_fall_occurred;
pub use fct::{
    ArmorBrokenFct, BleedingFct, CombatLogEvent, CombatLogSource, CombatLogSourceAppExt,
    CombatLogSystems, CombatText, ConsequenceFct, ConsequenceFctAppExt, ConsequenceFctSystems,
    ConsequencePop, DotFct, FctAnchorCell, FctEmphasis, FctSlotAllocator, FctStackIndex,
    FctValence, FieldFct, FloatingCombatText, InjuryFct, InjuryLogText, LogLine, LogName,
    OnDeathFct, PopAnchor, SuppressionFct, animate_floating_text, classify_log_event,
    forward_live_log_source, forward_log_source, forward_turn_started, read_consequence_fct,
    register_consequence_fct_core, severity_color, spawn_floating_text, valence_color,
};
pub use flash::{FlashTtl, FxFlash, expire_flashes};
pub use impact::{ShotImpactResolved, animate_impact};
pub use melee::read_melee_resolved;
pub use projectile::{
    PendingImpact, ProjectileTravel, ShotProjectile, advance_projectiles, spawn_shot_projectiles,
};
pub use readers::{read_armor_broken, read_bleeding, read_cover_destroyed};
pub use registrar::FxReaderAppExt;
pub(crate) use roles::register_effect_roles_hot_ron;
pub use roles::{
    COMPASS_DIRECTIONS, DIRECTION_COUNT, DamageTypeFx, EffectRoles, IMPACT_FRAME_COUNT,
    nearest_direction_index,
};
pub(crate) use tuning::register_fx_tuning_hot_ron;
pub use tuning::{
    FctRiseRate, FctTtlSeconds, FxTuning, ImpactFrameSeconds, InterShotSeconds,
    ProjectileDrawScale, ProjectileVelocity,
};
