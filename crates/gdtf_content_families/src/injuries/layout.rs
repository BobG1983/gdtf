//! Folder layout and file extensions for injury content.

use gdtf_battle_sim::{armor::InjuryCategory, injuries::DamageContext};

/// Root folder for injury content.
pub const INJURIES_FOLDER: &str = "content/injuries";

/// Extension for injury definition files.
pub const INJURY_DEF_EXTENSION: &str = "injury.ron";

/// Extension for injury weighting files.
pub const INJURY_WEIGHTING_EXTENSION: &str = "weighting.ron";

/// Subfolder name for weightings under a category.
pub const WEIGHTING_SUBFOLDER: &str = "weighting";

/// Directory name for a body-part category.
#[must_use]
pub const fn category_dir(category: InjuryCategory) -> &'static str {
    match category {
        InjuryCategory::Head => "head",
        InjuryCategory::Torso => "torso",
        InjuryCategory::Arm => "arm",
        InjuryCategory::Leg => "leg",
    }
}

/// Optional path infix for a damage context weighting folder.
#[must_use]
pub const fn weighting_context_infix(context: DamageContext) -> Option<&'static str> {
    match context {
        DamageContext::Ranged => None,
        DamageContext::Melee => Some("melee"),
        DamageContext::Fall => Some("fall"),
    }
}
