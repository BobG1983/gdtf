use gdtf_battle_sim::{
    armor::InjuryCategory,
    injuries::{DamageContext, InjuryName, InjuryRegistry, InjuryTables},
};

use crate::injury_form::{InjuryDraft, WeightingDraft};

pub(crate) fn autoload_first_injury(draft: &mut InjuryDraft, registry: &InjuryRegistry) {
    if !draft.autoload_pending() {
        return;
    }
    let mut keys: Vec<&InjuryName> = registry.iter().map(|(key, _)| key).collect();
    keys.sort();
    match keys
        .first()
        .and_then(|key| registry.def(key).map(|def| ((*key).clone(), def.clone())))
    {
        Some((key, def)) => draft.load_injury(&key, &def),
        None => draft.mark_autoloaded(),
    }
}

pub(crate) fn autoload_weighting_table(draft: &mut WeightingDraft, tables: &InjuryTables) {
    if !draft.autoload_pending() {
        return;
    }
    draft.load_table(InjuryCategory::ALL[0], DamageContext::ALL[0], tables);
}

#[cfg(test)]
mod tests {
    use gdtf_battle_sim::{
        armor::InjuryCategory,
        injuries::{
            DamageContext, InjuryDef, InjuryName, InjuryRegistry, InjuryTables, InjuryWeight,
            WeightedInjuryEntry, WeightedInjuryTable,
        },
        severity::Severity,
    };

    use super::{autoload_first_injury, autoload_weighting_table};
    use crate::injury_form::{InjuryDraft, WeightingDraft};

    fn fixture_def(name: &str) -> InjuryDef {
        let ron = format!(
            "(name: \"{name}\", category: Leg, severity: Minor, popup_text: \"X\", \
             log_text: \"x\", inspect_text: \"x\", effects: [DisableHand])",
        );
        let parsed = ron::de::from_str::<InjuryDef>(&ron);
        assert!(parsed.is_ok(), "fixture def must parse: {parsed:?}");
        let Ok(def) = parsed else {
            unreachable!("asserted Ok above")
        };
        def
    }

    #[test]
    fn seeds_first_sorted_injury_exactly_once() {
        let registry = InjuryRegistry::new([
            (
                InjuryName::new("torn_muscle".to_owned()),
                fixture_def("Torn Muscle"),
            ),
            (
                InjuryName::new("broken_nose".to_owned()),
                fixture_def("Broken Nose"),
            ),
        ]);
        let mut draft = InjuryDraft::default();
        autoload_first_injury(&mut draft, &registry);
        assert_eq!(draft.key(), "broken_nose", "sorted-first pick");

        draft.set_key("renamed".to_owned());
        autoload_first_injury(&mut draft, &registry);
        assert_eq!(draft.key(), "renamed");

        let mut empty_seeded = InjuryDraft::default();
        autoload_first_injury(&mut empty_seeded, &InjuryRegistry::default());
        assert!(!empty_seeded.autoload_pending());
        assert_eq!(empty_seeded.key(), "");
    }

    #[test]
    fn seeds_first_category_table_exactly_once() {
        let row = WeightedInjuryEntry::new(
            InjuryName::new("broken_nose".to_owned()),
            InjuryWeight::new(7),
        );
        let tables = InjuryTables::new([(
            (InjuryCategory::Head, DamageContext::Ranged, Severity::Minor),
            WeightedInjuryTable::new(vec![row.clone()]),
        )]);
        let mut draft = WeightingDraft::default();
        autoload_weighting_table(&mut draft, &tables);
        assert_eq!(draft.category(), InjuryCategory::ALL[0]);
        assert_eq!(draft.weighting().minor, vec![row]);
        assert_eq!(draft.weighting().major, []);

        draft.load_table(InjuryCategory::Leg, DamageContext::Ranged, &tables);
        autoload_weighting_table(&mut draft, &tables);
        assert_eq!(draft.category(), InjuryCategory::Leg);
    }
}
