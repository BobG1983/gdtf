//! The MELEE-WEAPON mode's ONE-SHOT open-with-a-weapon seed (GTW-671) — the parity twin
//! of the Gang / Armor / Injury / Sprite / Attachment / Weapon autoloads, run by the
//! shell on the first MeleeWeapon-mode frame.

use gdtf_battle_sim::weapon::{MeleeWeaponRegistry, WeaponName};

use crate::melee_weapon_form::MeleeWeaponDraft;

/// Seed a still-pristine [`MeleeWeaponDraft`] from the resolved
/// [`MeleeWeaponRegistry`] — the FIRST melee weapon by sorted [`WeaponName`] (registry
/// iteration order is unspecified, so the keys are sorted for a deterministic pick —
/// the Gang / Armor / Weapon modes' exact open behavior), or leave the form empty when
/// no melee weapons are loaded. Either way the one-shot seed is marked done, so it
/// never clobbers later edits / a deliberate "New melee weapon" (idempotent under the
/// egui multipass re-run — the first pass ends the pending state).
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
        // No melee weapons loaded — start empty (the Gang / Armor modes'
        // empty-registry branch).
        None => draft.mark_autoloaded(),
    }
}

#[cfg(test)]
mod tests {
    use gdtf_battle_sim::weapon::{MeleeWeaponRegistry, WeaponName};

    use super::autoload_first_melee_weapon;
    use crate::melee_weapon_form::MeleeWeaponDraft;

    /// The one-shot seed loads the FIRST melee weapon by sorted key; a second call is a
    /// no-op (multipass idempotency); an empty registry just ends the pending state.
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

        // A later edit is never clobbered by a re-run.
        draft.set_name("renamed".to_owned());
        autoload_first_melee_weapon(&mut draft, &registry);
        assert_eq!(draft.name(), "renamed");

        let mut empty_seeded = MeleeWeaponDraft::default();
        autoload_first_melee_weapon(&mut empty_seeded, &MeleeWeaponRegistry::default());
        assert!(!empty_seeded.autoload_pending());
        assert_eq!(empty_seeded.name(), "");
    }
}
