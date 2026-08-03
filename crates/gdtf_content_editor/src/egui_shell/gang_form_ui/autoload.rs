use gdtf_battle_sim::ganger::{GangName, GangRegistry};

use crate::gang_form::GangDraft;

pub(crate) fn autoload_first_gang(draft: &mut GangDraft, registry: &GangRegistry) {
    if !draft.autoload_pending() {
        return;
    }
    let mut names: Vec<&GangName> = registry.keys().collect();
    names.sort_by(|a, b| a.as_str().cmp(b.as_str()));
    match names.first().and_then(|name| {
        registry
            .roster(name)
            .map(|roster| ((*name).clone(), roster.clone()))
    }) {
        Some((name, roster)) => draft.load_gang(&name, &roster),
        None => draft.mark_autoloaded(),
    }
}

#[cfg(test)]
mod tests {
    use gdtf_battle_sim::ganger::{GangName, GangRegistry, GangRoster};

    use super::autoload_first_gang;
    use crate::gang_form::GangDraft;

            #[test]
    fn seeds_first_sorted_gang_exactly_once() {
        let registry = GangRegistry::new([
            (GangName::new("zeta".to_owned()), GangRoster::default()),
            (GangName::new("alpha".to_owned()), GangRoster::default()),
        ]);
        let mut draft = GangDraft::default();
        autoload_first_gang(&mut draft, &registry);
        assert_eq!(draft.name(), "alpha", "sorted-first pick");

        draft.set_name("renamed".to_owned());
        autoload_first_gang(&mut draft, &registry);
        assert_eq!(draft.name(), "renamed");

        let mut empty_seeded = GangDraft::default();
        autoload_first_gang(&mut empty_seeded, &GangRegistry::default());
        assert!(!empty_seeded.autoload_pending());
        assert_eq!(empty_seeded.name(), "");
    }
}
