//! The WEAPON mode's ONE-SHOT open-with-a-weapon seed (GTW-670) — the parity twin of
//! the Gang / Armor / Injury / Sprite / Attachment autoloads, run by the shell on the
//! first Weapon-mode frame.

use gdtf_battle_sim::weapon::{WeaponName, WeaponRegistry};

use crate::weapon_form::WeaponDraft;

/// Seed a still-pristine [`WeaponDraft`] from the resolved [`WeaponRegistry`] — the
/// FIRST weapon by sorted [`WeaponName`] (registry iteration order is unspecified, so
/// the keys are sorted for a deterministic pick — the Gang / Armor / Attachment modes'
/// exact open behavior), or leave the form empty when no weapons are loaded. Either way
/// the one-shot seed is marked done, so it never clobbers later edits / a deliberate
/// "New weapon" (idempotent under the egui multipass re-run — the first pass ends the
/// pending state).
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
        // No weapons loaded — start empty (the Gang / Armor modes' empty-registry
        // branch).
        None => draft.mark_autoloaded(),
    }
}

#[cfg(test)]
mod tests {
    use gdtf_battle_sim::weapon::{WeaponName, WeaponRegistry};

    use super::autoload_first_weapon;
    use crate::weapon_form::WeaponDraft;

    /// The one-shot seed loads the FIRST weapon by sorted key; a second call is a no-op
    /// (multipass idempotency); an empty registry just ends the pending state.
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

        // A later edit is never clobbered by a re-run.
        draft.set_name("renamed".to_owned());
        autoload_first_weapon(&mut draft, &registry);
        assert_eq!(draft.name(), "renamed");

        let mut empty_seeded = WeaponDraft::default();
        autoload_first_weapon(&mut empty_seeded, &WeaponRegistry::default());
        assert!(!empty_seeded.autoload_pending());
        assert_eq!(empty_seeded.name(), "");
    }
}
