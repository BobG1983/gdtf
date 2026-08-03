use gdtf_battle_sim::weapon::{WeaponName, WeaponRegistry};

use crate::weapon_form::WeaponDraft;

pub(crate) fn autoload_first_weapon(draft: &mut WeaponDraft, registry: &WeaponRegistry) {
    if !draft.autoload_pending() {
        return;
    }
    let mut names: Vec<&WeaponName> = registry.keys().collect();
    names.sort_by(|a, b| a.as_str().cmp(b.as_str()));
    match names.first().and_then(|name| {
        registry
            .spec(name)
            .map(|spec| ((*name).clone(), spec.clone()))
    }) {
        Some((name, spec)) => draft.load_weapon(&name, &spec),
        None => draft.mark_autoloaded(),
    }
}

#[cfg(test)]
mod tests {
    use gdtf_battle_sim::weapon::{WeaponName, WeaponRegistry};

    use super::autoload_first_weapon;
    use crate::weapon_form::WeaponDraft;

            #[test]
    fn seeds_first_sorted_weapon_exactly_once() {
        let seed = WeaponDraft::new_weapon();
        let registry = WeaponRegistry::new([
            (
                WeaponName::new("stub_pistol".to_owned()),
                seed.spec().clone(),
            ),
            (
                WeaponName::new("arc_pistol".to_owned()),
                seed.spec().clone(),
            ),
        ]);
        let mut draft = WeaponDraft::default();
        autoload_first_weapon(&mut draft, &registry);
        assert_eq!(draft.name(), "arc_pistol", "sorted-first pick");

        draft.set_name("renamed".to_owned());
        autoload_first_weapon(&mut draft, &registry);
        assert_eq!(draft.name(), "renamed");

        let mut empty_seeded = WeaponDraft::default();
        autoload_first_weapon(&mut empty_seeded, &WeaponRegistry::default());
        assert!(!empty_seeded.autoload_pending());
        assert_eq!(empty_seeded.name(), "");
    }
}
