//! The ARMOR mode's ONE-SHOT open-with-an-armor seed (GTW-479) — the parity twin of the
//! Gang mode's open-with-a-gang autoload (GTW-636), run by the shell on the first
//! Armor-mode frame.

use gdtf_battle_sim::armor::{ArmorName, ArmorRegistry};

use crate::armor_form::ArmorDraft;

/// Seed a still-pristine [`ArmorDraft`] from the resolved [`ArmorRegistry`] — the FIRST
/// armor by sorted [`ArmorName`] (registry iteration order is unspecified, so the keys
/// are sorted for a deterministic pick — the Gang mode's exact open behavior), or leave
/// the form empty when no armor is loaded. Either way the one-shot seed is marked done,
/// so it never clobbers later edits / a deliberate "New armor" (idempotent under the
/// egui multipass re-run — the first pass ends the pending state).
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
        // No armor loaded — start empty (the Gang mode's empty-registry branch).
        None => draft.mark_autoloaded(),
    }
}

#[cfg(test)]
mod tests {
    use gdtf_battle_sim::armor::{ArmorName, ArmorRegistry, ArmorSpec};

    use super::autoload_first_armor;
    use crate::armor_form::ArmorDraft;

    /// The one-shot seed loads the FIRST armor by sorted name; a second call is a no-op
    /// (multipass idempotency); an empty registry just ends the pending state.
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

        // A later edit is never clobbered by a re-run.
        draft.set_name("renamed".to_owned());
        autoload_first_armor(&mut draft, &registry);
        assert_eq!(draft.name(), "renamed");

        let mut empty_seeded = ArmorDraft::default();
        autoload_first_armor(&mut empty_seeded, &ArmorRegistry::default());
        assert!(!empty_seeded.autoload_pending());
        assert_eq!(empty_seeded.name(), "");
    }
}
