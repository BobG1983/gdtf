//! The one spelling of an injury weighting table's content member key.

use gdtf_assets::ContentMemberKey;
use gdtf_battle_sim::{armor::InjuryCategory, injuries::DamageContext};

/// The key one weighting table is named by: its category and its damage context.
#[must_use]
pub fn weighting_member_key(category: InjuryCategory, context: DamageContext) -> ContentMemberKey {
    ContentMemberKey::new(format!("{category:?}/{context:?}"))
}

/// Read the category and damage context back out of a weighting table's key.
#[must_use]
pub fn weighting_key_parts(key: &ContentMemberKey) -> Option<(InjuryCategory, DamageContext)> {
    InjuryCategory::ALL.into_iter().find_map(|category| {
        DamageContext::ALL
            .into_iter()
            .find(|context| *weighting_member_key(category, *context) == **key)
            .map(|context| (category, context))
    })
}
