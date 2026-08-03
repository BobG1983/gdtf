//! The INJURIES family's authored-content layout vocabulary (GTW-634 C4) — the
use gdtf_battle_sim::{armor::InjuryCategory, injuries::DamageContext};

pub const INJURIES_FOLDER: &str = "content/injuries";

pub const INJURY_DEF_EXTENSION: &str = "injury.ron";

pub const INJURY_WEIGHTING_EXTENSION: &str = "weighting.ron";

pub const WEIGHTING_SUBFOLDER: &str = "weighting";

#[must_use]
pub const fn category_dir(category: InjuryCategory) -> &'static str {
    match category {
        InjuryCategory::Head => "head",
        InjuryCategory::Torso => "torso",
        InjuryCategory::Arm => "arm",
        InjuryCategory::Leg => "leg",
    }
}

#[must_use]
pub const fn weighting_context_infix(context: DamageContext) -> Option<&'static str> {
    match context {
        DamageContext::Ranged => None,
        DamageContext::Melee => Some("melee"),
        DamageContext::Fall => Some("fall"),
    }
}
