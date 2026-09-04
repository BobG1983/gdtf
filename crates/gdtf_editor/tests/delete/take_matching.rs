//! `take_matching_assets` removes the entries its predicate matches, and only those.

use bevy::asset::Assets;
use cobalt_ron_assets::RonAsset;
use gdtf_battle_sim::{
    armor::InjuryCategory,
    injuries::{DamageContext, InjuryWeighting},
};
use gdtf_editor::take_matching_assets;

/// The context both stored weightings share, so only the category tells them apart.
const CONTEXT: DamageContext = DamageContext::Ranged;

/// An empty weighting table for one category, in the shared context.
const fn weighting(category: InjuryCategory) -> InjuryWeighting {
    InjuryWeighting {
        category,
        context: CONTEXT,
        minor: Vec::new(),
        major: Vec::new(),
        critical: Vec::new(),
    }
}

#[test]
fn taking_matching_assets_leaves_the_entries_the_predicate_misses() {
    let head = weighting(InjuryCategory::Head);
    let torso = weighting(InjuryCategory::Torso);

    let mut assets = Assets::<RonAsset<InjuryWeighting>>::default();
    assets.add(RonAsset::new(head.clone()));
    assets.add(RonAsset::new(torso.clone()));

    let taken = take_matching_assets(&mut assets, |stored| {
        stored.category == InjuryCategory::Head
    });

    assert_eq!(
        taken.len(),
        1,
        "the predicate names one of the two stored weightings, so the take must hand back that \
         one alone",
    );
    let Some(first) = taken.first() else { return };
    assert_eq!(
        &**first, &head,
        "the returned asset must be the Head weighting the predicate matched",
    );
    assert_eq!(
        assets.iter().count(),
        1,
        "the unmatched Torso weighting must stay in the store — a take that ignores the \
         predicate empties it",
    );
    let survivor = assets.iter().next().map(|(_id, asset)| (**asset).clone());
    assert_eq!(
        survivor,
        Some(torso),
        "the surviving entry must be the Torso weighting, untouched by the Head take",
    );
}
