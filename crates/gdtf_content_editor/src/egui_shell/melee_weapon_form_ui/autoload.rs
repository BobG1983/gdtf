use gdtf_battle_sim::weapon::{MeleeWeaponRegistry, WeaponName};

use crate::melee_weapon_form::MeleeWeaponDraft;

pub(crate) fn autoload_first_melee_weapon(
    draft: &mut MeleeWeaponDraft,
    registry: &MeleeWeaponRegistry,
) {
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
        Some((name, spec)) => draft.load_melee_weapon(&name, &spec),
        None => draft.mark_autoloaded(),
    }
}

#[cfg(test)]
mod tests {
    use gdtf_battle_sim::weapon::{MeleeWeaponRegistry, WeaponName};

    use super::autoload_first_melee_weapon;
    use crate::melee_weapon_form::MeleeWeaponDraft;

            #[test]
    fn seeds_first_sorted_melee_weapon_exactly_once() {
        let seed = MeleeWeaponDraft::new_melee_weapon();
        let registry = MeleeWeaponRegistry::new([
            (WeaponName::new("fists".to_owned()), seed.spec().clone()),
            (
                WeaponName::new("chainsword".to_owned()),
                seed.spec().clone(),
            ),
        ]);
        let mut draft = MeleeWeaponDraft::default();
        autoload_first_melee_weapon(&mut draft, &registry);
        assert_eq!(draft.name(), "chainsword", "sorted-first pick");

        draft.set_name("renamed".to_owned());
        autoload_first_melee_weapon(&mut draft, &registry);
        assert_eq!(draft.name(), "renamed");

        let mut empty_seeded = MeleeWeaponDraft::default();
        autoload_first_melee_weapon(&mut empty_seeded, &MeleeWeaponRegistry::default());
        assert!(!empty_seeded.autoload_pending());
        assert_eq!(empty_seeded.name(), "");
    }
}
