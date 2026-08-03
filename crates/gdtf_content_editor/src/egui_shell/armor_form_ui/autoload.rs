use gdtf_battle_sim::armor::{ArmorName, ArmorRegistry};

use crate::armor_form::ArmorDraft;

pub(crate) fn autoload_first_armor(draft: &mut ArmorDraft, registry: &ArmorRegistry) {
    if !draft.autoload_pending() {
        return;
    }
    let mut names: Vec<&ArmorName> = registry.keys().collect();
    names.sort_by(|a, b| a.as_str().cmp(b.as_str()));
    match names
        .first()
        .and_then(|name| registry.spec(name).map(|spec| ((*name).clone(), *spec)))
    {
        Some((name, spec)) => draft.load_armor(&name, &spec),
        None => draft.mark_autoloaded(),
    }
}

#[cfg(test)]
mod tests {
    use gdtf_battle_sim::armor::{ArmorName, ArmorRegistry, ArmorSpec};

    use super::autoload_first_armor;
    use crate::armor_form::ArmorDraft;

            #[test]
    fn seeds_first_sorted_armor_exactly_once() {
        let registry = ArmorRegistry::new([
            (
                ArmorName::new("zeta_plate".to_owned()),
                ArmorSpec::uniform(gdtf_battle_sim::armor::ArmorPiece::default()),
            ),
            (
                ArmorName::new("alpha_vest".to_owned()),
                ArmorSpec::uniform(gdtf_battle_sim::armor::ArmorPiece::default()),
            ),
        ]);
        let mut draft = ArmorDraft::default();
        autoload_first_armor(&mut draft, &registry);
        assert_eq!(draft.name(), "alpha_vest", "sorted-first pick");

        draft.set_name("renamed".to_owned());
        autoload_first_armor(&mut draft, &registry);
        assert_eq!(draft.name(), "renamed");

        let mut empty_seeded = ArmorDraft::default();
        autoload_first_armor(&mut empty_seeded, &ArmorRegistry::default());
        assert!(!empty_seeded.autoload_pending());
        assert_eq!(empty_seeded.name(), "");
    }
}
