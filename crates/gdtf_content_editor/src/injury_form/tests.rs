use gdtf_assets::serialize_ron_pretty;
use gdtf_battle_sim::{
    armor::InjuryCategory,
    injuries::{
        DamageContext, InjuryDef, InjuryEffect, InjuryName, InjuryTables, InjuryWeight,
        InjuryWeighting, StatDelta, StatTarget, WeightedInjuryEntry, WeightedInjuryTable,
    },
    severity::Severity,
};
use gdtf_content_families::injuries::{
    INJURIES_FOLDER, INJURY_DEF_EXTENSION, INJURY_WEIGHTING_EXTENSION, WEIGHTING_SUBFOLDER,
};

use super::{
    draft::InjuryDraft,
    save::{
        draft_to_def, draft_to_weighting, injury_file_name, injury_save_path_in,
        weighting_file_name, weighting_save_path_in,
    },
    weighting::WeightingDraft,
};

fn fixture_def() -> InjuryDef {
    let ron = "(name: \"Test Wound\", category: Arm, severity: Major, \
               popup_text: \"WOUNDED\", log_text: \"is wounded\", \
               inspect_text: \"Test Wound -- fixture\", \
               effects: [Modify(stat: Aim, amount: -2), Bleeding(amount: 1), \
                         DisableHand, MovementCostMul(1.5)])";
    let parsed = ron::de::from_str::<InjuryDef>(ron);
    assert!(parsed.is_ok(), "fixture def must parse: {parsed:?}");
    let Ok(def) = parsed else {
        unreachable!("asserted Ok above")
    };
    def
}

#[test]
fn default_is_pristine_and_load_injury_fills_the_form() {
    let mut draft = InjuryDraft::default();
    assert!(
        draft.autoload_pending(),
        "the fresh seed must autoload once"
    );
    assert_eq!(draft.key(), "");
    assert_eq!(
        draft.def().effects.len(),
        1,
        "a pristine def carries one seed effect row (the schema's `effects ≥ 1`)",
    );

    let def = fixture_def();
    draft.load_injury(&InjuryName::new("test_wound".to_owned()), &def);
    assert!(
        !draft.autoload_pending(),
        "a loaded injury ends the autoload"
    );
    assert_eq!(draft.key(), "test_wound");
    assert_eq!(draft.def(), &def);

    let minted = InjuryDraft::new_injury();
    assert!(
        !minted.autoload_pending(),
        "a deliberate new injury never re-seeds"
    );
    assert_eq!(minted.key(), "");

    let mut pristine = InjuryDraft::default();
    pristine.mark_autoloaded();
    assert!(
        !pristine.autoload_pending(),
        "the empty-registry branch ends the seed"
    );
}

#[test]
fn edited_def_round_trips_through_the_loader_schema() {
    let mut edited = InjuryDraft::new_injury();
    edited.load_injury(&InjuryName::new("test_wound".to_owned()), &fixture_def());
    edited.def_mut().effects.push(InjuryEffect::Modify {
        stat:   StatTarget::Grit,
        amount: StatDelta::new(3),
    });

    let (key, def) = draft_to_def(&edited);
    let serialized = serialize_ron_pretty(&def);
    assert!(
        serialized.is_ok(),
        "serializing the edited def must succeed: {:?}",
        serialized.as_ref().err(),
    );
    let Ok(serialized) = serialized else { return };

    let reloaded_def = ron::de::from_str::<InjuryDef>(&serialized);
    assert!(
        reloaded_def.is_ok(),
        "the serialized def must round-trip through the InjuryDef deserializer: {:?}",
        reloaded_def.as_ref().err(),
    );
    let Ok(reloaded_def) = reloaded_def else {
        return;
    };

    let mut reloaded = InjuryDraft::new_injury();
    reloaded.load_injury(&key, &reloaded_def);
    assert_eq!(
        reloaded, edited,
        "the reloaded injury must equal the edited draft (key + every def field incl. \
         the five effect rows) — the GTW-437 stem-key round-trip",
    );
}

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

#[test]
fn save_file_names_and_paths_derive_from_the_one_owner_spellings() {
    let key = InjuryName::new("twisted_ankle".to_owned());
    assert_eq!(
        injury_file_name(&key),
        format!("twisted_ankle.{INJURY_DEF_EXTENSION}"),
    );

    let hostile = InjuryName::new("../../Evil Wound!".to_owned());
    assert_eq!(injury_file_name(&hostile), "evil_wound.injury.ron");
    let path = injury_save_path_in(
        std::path::Path::new("/tmp/root"),
        InjuryCategory::Leg,
        &hostile,
    );
    let expected_tail = std::path::Path::new(INJURIES_FOLDER)
        .join("leg")
        .join("evil_wound.injury.ron");
    assert!(
        path.ends_with(&expected_tail),
        "the sanitized def lands under its category subfolder: {path:?}",
    );

    let unnameable = InjuryName::new("!!!///".to_owned());
    assert_eq!(injury_file_name(&unnameable), "unnamed_injury.injury.ron");

    assert_eq!(
        weighting_file_name(InjuryCategory::Torso, DamageContext::Ranged),
        format!("torso.{INJURY_WEIGHTING_EXTENSION}"),
    );
    assert_eq!(
        weighting_file_name(InjuryCategory::Torso, DamageContext::Melee),
        format!("torso.melee.{INJURY_WEIGHTING_EXTENSION}"),
        "a per-source pick must save to its OWN file, never overwrite the ranged one",
    );
    let weighting_path = weighting_save_path_in(
        std::path::Path::new("/tmp/root"),
        InjuryCategory::Torso,
        DamageContext::Ranged,
    );
    let expected_tail = std::path::Path::new(INJURIES_FOLDER)
        .join(WEIGHTING_SUBFOLDER)
        .join("torso.weighting.ron");
    assert!(
        weighting_path.ends_with(&expected_tail),
        "the weighting table lands under the weighting subfolder: {weighting_path:?}",
    );

    assert!(matches!(
        super::draft::DEFAULT_EFFECT,
        InjuryEffect::Modify { .. }
    ));
}
