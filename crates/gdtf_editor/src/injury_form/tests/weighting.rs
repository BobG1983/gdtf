use cobalt_ron_assets::serialize_ron_pretty;
use gdtf_battle_sim::{
    armor::InjuryCategory,
    injuries::{
        DamageContext, InjuryName, InjuryTables, InjuryWeight, InjuryWeighting,
        WeightedInjuryEntry, WeightedInjuryTable,
    },
    severity::Severity,
};

use crate::injury_form::{save::draft_to_weighting, weighting::WeightingDraft};

fn fixture_tables() -> InjuryTables {
    let row = |key: &str, weight: u32| {
        WeightedInjuryEntry::new(InjuryName::new(key.to_owned()), InjuryWeight::new(weight))
    };
    InjuryTables::new([
        (
            (InjuryCategory::Arm, DamageContext::Ranged, Severity::Minor),
            WeightedInjuryTable::new(vec![row("bruise", 4), row("sprain", 2)]),
        ),
        (
            (
                InjuryCategory::Arm,
                DamageContext::Ranged,
                Severity::Critical,
            ),
            WeightedInjuryTable::new(vec![row("mangled", 1)]),
        ),
        (
            (InjuryCategory::Leg, DamageContext::Ranged, Severity::Minor),
            WeightedInjuryTable::new(vec![row("limp", 9)]),
        ),
    ])
}

#[test]
fn load_table_fills_exactly_the_picked_category_and_context() {
    let tables = fixture_tables();
    let mut draft = WeightingDraft::default();
    assert!(
        draft.autoload_pending(),
        "the fresh seed must autoload once"
    );

    draft.load_table(InjuryCategory::Arm, DamageContext::Ranged, &tables);
    assert!(!draft.autoload_pending());
    assert_eq!(draft.category(), InjuryCategory::Arm);
    assert_eq!(
        draft.weighting().minor.len(),
        2,
        "the Arm Minor bucket rows"
    );
    assert!(
        draft.weighting().major.is_empty(),
        "an unauthored bucket loads empty"
    );
    assert_eq!(draft.weighting().critical.len(), 1);
    assert!(
        !draft
            .weighting()
            .minor
            .iter()
            .any(|entry| entry.injury.as_str() == "limp"),
        "another category's rows must not leak in",
    );

    draft.weighting_mut().major.push(WeightedInjuryEntry::new(
        InjuryName::new("bruise".to_owned()),
        InjuryWeight::new(5),
    ));
    assert_eq!(draft.weighting().major.len(), 1);
    assert_eq!(
        draft.weighting().minor.len(),
        2,
        "sibling buckets untouched"
    );
}

#[test]
fn edited_weighting_round_trips_through_the_loader_schema() {
    let mut edited = WeightingDraft::default();
    edited.load_table(
        InjuryCategory::Arm,
        DamageContext::Ranged,
        &fixture_tables(),
    );
    edited
        .weighting_mut()
        .critical
        .push(WeightedInjuryEntry::new(
            InjuryName::new("sprain".to_owned()),
            InjuryWeight::new(7),
        ));

    let weighting = draft_to_weighting(&edited);
    let serialized = serialize_ron_pretty(&weighting);
    assert!(
        serialized.is_ok(),
        "serializing the edited weighting must succeed: {:?}",
        serialized.as_ref().err(),
    );
    let Ok(serialized) = serialized else { return };

    let reloaded = ron::de::from_str::<InjuryWeighting>(&serialized);
    assert!(
        reloaded.is_ok(),
        "the serialized weighting must round-trip through the InjuryWeighting \
         deserializer: {:?}",
        reloaded.as_ref().err(),
    );
    assert_eq!(
        reloaded.ok().as_ref(),
        Some(&weighting),
        "the reloaded weighting must equal the edited record (category + all three \
         buckets' rows)",
    );
}
