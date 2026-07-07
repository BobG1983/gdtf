//! The GANG mode's ONE-SHOT open-with-a-gang seed (GTW-636) — the parity twin of the
//! retired in-game editor's `OnEnter` model load, run by the shell on the first
//! Gang-mode frame (the theme form's C3.2 autoload precedent).

use gdtf_battle_sim::ganger::{GangName, GangRegistry};

use crate::gang_form::GangDraft;

/// Seed a still-pristine [`GangDraft`] from the resolved [`GangRegistry`] — the FIRST
/// gang by sorted [`GangName`] (registry iteration order is unspecified, so the keys are
/// sorted for a deterministic pick — the retired editor's exact open behavior), or
/// leave the form empty when no gangs are loaded. Either way the one-shot seed is
/// marked done, so it never clobbers later edits / a deliberate "New gang" (idempotent
/// under the egui multipass re-run — the first pass ends the pending state).
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
        // No gangs loaded — start empty (the retired editor's AC2 branch).
        None => draft.mark_autoloaded(),
    }
}

#[cfg(test)]
mod tests {
    use gdtf_battle_sim::ganger::{GangName, GangRegistry, GangRoster};

    use super::autoload_first_gang;
    use crate::gang_form::GangDraft;

    /// The one-shot seed loads the FIRST gang by sorted name; a second call is a no-op
    /// (multipass idempotency); an empty registry just ends the pending state.
    #[test]
    fn seeds_first_sorted_gang_exactly_once() {
        let registry = GangRegistry::new([
            (GangName::new("zeta".to_owned()), GangRoster::default()),
            (GangName::new("alpha".to_owned()), GangRoster::default()),
        ]);
        let mut draft = GangDraft::default();
        autoload_first_gang(&mut draft, &registry);
        assert_eq!(draft.name(), "alpha", "sorted-first pick");

        // A later edit is never clobbered by a re-run.
        draft.set_name("renamed".to_owned());
        autoload_first_gang(&mut draft, &registry);
        assert_eq!(draft.name(), "renamed");

        let mut empty_seeded = GangDraft::default();
        autoload_first_gang(&mut empty_seeded, &GangRegistry::default());
        assert!(!empty_seeded.autoload_pending());
        assert_eq!(empty_seeded.name(), "");
    }
}
