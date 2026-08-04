use gdtf_content_families::sprites::{SpriteDefRegistry, SpriteName};

use crate::sprite_form::SpriteDraft;

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

        draft.set_name("renamed".to_owned());
        autoload_first_sprite(&mut draft, &registry);
        assert_eq!(draft.name(), "renamed");

        let mut empty_seeded = SpriteDraft::default();
        autoload_first_sprite(&mut empty_seeded, &SpriteDefRegistry::default());
        assert!(!empty_seeded.autoload_pending());
        assert_eq!(empty_seeded.name(), "");
    }
}
