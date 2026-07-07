//! The SPRITE mode's ONE-SHOT open-with-a-sprite seed (GTW-664) — the parity twin of
//! the Gang / Armor / Injury autoloads, run by the shell on the first Sprite-mode frame.

use gdtf_content_families::sprites::{SpriteDefRegistry, SpriteName};

use crate::sprite_form::SpriteDraft;

/// Seed a still-pristine [`SpriteDraft`] from the resolved [`SpriteDefRegistry`] — the
/// FIRST def by sorted [`SpriteName`] (registry iteration order is unspecified, so the
/// keys are sorted for a deterministic pick — the Gang / Armor modes' exact open
/// behavior), or leave the form empty when no defs are loaded. Either way the one-shot
/// seed is marked done, so it never clobbers later edits / a deliberate "New sprite"
/// (idempotent under the egui multipass re-run — the first pass ends the pending state).
pub(crate) fn autoload_first_sprite(draft: &mut SpriteDraft, registry: &SpriteDefRegistry) {
    if !draft.autoload_pending() {
        return;
    }
    let mut names: Vec<&SpriteName> = registry.keys().collect();
    names.sort_by(|a, b| a.as_str().cmp(b.as_str()));
    match names
        .first()
        .and_then(|name| registry.def(name).map(|def| ((*name).clone(), def.clone())))
    {
        Some((name, def)) => draft.load_sprite(&name, &def),
        // No defs loaded — start empty (the Gang / Armor modes' empty-registry branch).
        None => draft.mark_autoloaded(),
    }
}

#[cfg(test)]
mod tests {
    use gdtf_content_families::sprites::{
        SpriteAnchor, SpriteDef, SpriteDefRegistry, SpriteImagePath, SpriteName, SpritePx,
        SpriteSource,
    };

    use super::autoload_first_sprite;
    use crate::sprite_form::SpriteDraft;

    /// A minimal named def for registry seeding.
    fn def(path: &str) -> SpriteDef {
        SpriteDef {
            source:    SpriteSource::File(SpriteImagePath::new(path.to_owned())),
            anchor:    SpriteAnchor {
                x: SpritePx::new(8),
                y: SpritePx::new(8),
            },
            facings:   None,
            animation: None,
        }
    }

    /// The one-shot seed loads the FIRST def by sorted name; a second call is a no-op
    /// (multipass idempotency); an empty registry just ends the pending state.
    #[test]
    fn seeds_first_sorted_sprite_exactly_once() {
        let registry = SpriteDefRegistry::new([
            (
                SpriteName::new("zeta_glow".to_owned()),
                def("sprites/z.png"),
            ),
            (
                SpriteName::new("alpha_vent".to_owned()),
                def("sprites/a.png"),
            ),
        ]);
        let mut draft = SpriteDraft::default();
        autoload_first_sprite(&mut draft, &registry);
        assert_eq!(draft.name(), "alpha_vent", "sorted-first pick");

        // A later edit is never clobbered by a re-run.
        draft.set_name("renamed".to_owned());
        autoload_first_sprite(&mut draft, &registry);
        assert_eq!(draft.name(), "renamed");

        let mut empty_seeded = SpriteDraft::default();
        autoload_first_sprite(&mut empty_seeded, &SpriteDefRegistry::default());
        assert!(!empty_seeded.autoload_pending());
        assert_eq!(empty_seeded.name(), "");
    }
}
