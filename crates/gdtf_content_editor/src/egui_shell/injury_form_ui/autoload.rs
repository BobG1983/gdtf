//! The INJURY mode's ONE-SHOT open-with-content seeds (GTW-654) — the parity twins
//! of the Gang/Armor open-with-content autoloads (GTW-636/GTW-479), run by the
//! shell on the first Injury-mode frame: the def form opens on the FIRST loaded
//! injury (sorted by key) and the weighting section on the FIRST canonical
//! category's current table.

use gdtf_battle_sim::{
    armor::InjuryCategory,
    injuries::{InjuryName, InjuryRegistry, InjuryTables},
};

use crate::injury_form::{InjuryDraft, WeightingDraft};

/// Seed a still-pristine [`InjuryDraft`] from the resolved [`InjuryRegistry`] — the
/// FIRST injury by sorted key (registry iteration order is unspecified, so the keys
/// are sorted for a deterministic pick — the Gang/Armor modes' exact open
/// behavior), or leave the form empty when no injury is loaded. Either way the
/// one-shot seed is marked done, so it never clobbers later edits / a deliberate
/// "New injury" (idempotent under the egui multipass re-run — the first pass ends
/// the pending state).
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
        // No injuries loaded — start empty (the Gang/Armor empty-registry branch).
        None => draft.mark_autoloaded(),
    }
}

/// Seed a still-pristine [`WeightingDraft`] from the resolved [`InjuryTables`] —
/// the FIRST canonical category's ([`InjuryCategory::ALL`]`[0]`) current table
/// (empty buckets when no weighting authored it: the table lifecycle is total, so
/// there is no separate empty branch). Idempotent under the egui multipass re-run.
pub(crate) fn autoload_weighting_table(draft: &mut WeightingDraft, tables: &InjuryTables) {
    if !draft.autoload_pending() {
        return;
    }
    draft.load_category(InjuryCategory::ALL[0], tables);
}

#[cfg(test)]
mod tests {
    use gdtf_battle_sim::{
        armor::InjuryCategory,
        injuries::{
            InjuryDef, InjuryName, InjuryRegistry, InjuryTables, InjuryWeight, WeightedInjuryEntry,
            WeightedInjuryTable,
        },
        severity::Severity,
    };

    use super::{autoload_first_injury, autoload_weighting_table};
    use crate::injury_form::{InjuryDraft, WeightingDraft};

    /// A minimal parseable def fixture (arbitrary magnitudes — never shipped pins).
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

    /// The one-shot seed loads the FIRST injury by sorted key; a second call is a
    /// no-op (multipass idempotency); an empty registry just ends the pending state.
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

        // A later edit is never clobbered by a re-run.
        draft.set_key("renamed".to_owned());
        autoload_first_injury(&mut draft, &registry);
        assert_eq!(draft.key(), "renamed");

        let mut empty_seeded = InjuryDraft::default();
        autoload_first_injury(&mut empty_seeded, &InjuryRegistry::default());
        assert!(!empty_seeded.autoload_pending());
        assert_eq!(empty_seeded.key(), "");
    }

    /// The weighting seed opens the FIRST canonical category (Head) with its
    /// current built bucket rows; a later context switch is never clobbered by a
    /// re-run.
    #[test]
    fn seeds_first_category_table_exactly_once() {
        let row = WeightedInjuryEntry::new(
            InjuryName::new("broken_nose".to_owned()),
            InjuryWeight::new(7),
        );
        let tables = InjuryTables::new([(
            (InjuryCategory::Head, Severity::Minor),
            WeightedInjuryTable::new(vec![row.clone()]),
        )]);
        let mut draft = WeightingDraft::default();
        autoload_weighting_table(&mut draft, &tables);
        assert_eq!(draft.category(), InjuryCategory::ALL[0]);
        assert_eq!(draft.weighting().minor, vec![row]);
        assert!(draft.weighting().major.is_empty());

        // A later context switch is not clobbered.
        draft.load_category(InjuryCategory::Leg, &tables);
        autoload_weighting_table(&mut draft, &tables);
        assert_eq!(draft.category(), InjuryCategory::Leg);
    }
}
