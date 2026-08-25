use gdtf_battle_sim::level::{ThemeUuid, UuidThemeDef, UuidThemeRegistry};

use crate::{mode::EditorMode, theme_form::ThemeDraft};

pub(crate) fn load_theme_into_form(draft: &mut ThemeDraft, def: &UuidThemeDef) {
    *draft = ThemeDraft::from_parts(
        def.key,
        (*def.display_name).clone(),
        def.terrain.clone(),
        def.default_floor,
    );
}

pub(crate) fn resolve_autoload(
    session_theme: ThemeUuid,
    themes: &UuidThemeRegistry,
) -> Option<&UuidThemeDef> {
    if *session_theme.is_nil() {
        return None;
    }
    themes.def(&session_theme)
}

/// Keep a New-theme blank, but still load when the session theme changes.
pub(crate) fn sync_theme_draft(
    mode: EditorMode,
    session_theme: ThemeUuid,
    themes: &UuidThemeRegistry,
    draft: &mut ThemeDraft,
) {
    if mode != EditorMode::Theme {
        return;
    }
    let Some(def) = resolve_autoload(session_theme, themes) else {
        return;
    };
    if draft.user_blank() {
        match draft.pinned_session() {
            Some(pinned) if pinned == session_theme => return,
            None => {
                draft.pin_session(session_theme);
                return;
            }
            Some(_) => {}
        }
    } else if draft.key() == def.key {
        return;
    }
    load_theme_into_form(draft, def);
}

#[cfg(test)]
mod tests {
    use gdtf_battle_sim::{
        level::{ThemeDisplayName, ThemeUuid, UuidThemeDef, UuidThemeRegistry},
        terrain::def::TerrainUuid,
    };

    use super::{load_theme_into_form, sync_theme_draft};
    use crate::{mode::EditorMode, theme_form::ThemeDraft};

    fn theme_key(n: u128) -> ThemeUuid {
        ThemeUuid::new(bevy::asset::uuid::Uuid::from_u128(0x0001_84c3_2000 + n))
    }

    fn terrain_key(n: u128) -> TerrainUuid {
        TerrainUuid::new(bevy::asset::uuid::Uuid::from_u128(0x0001_84c3_3000 + n))
    }

    fn theme_def(key: ThemeUuid, name: &str, floor: TerrainUuid) -> UuidThemeDef {
        UuidThemeDef {
            key,
            display_name: ThemeDisplayName::new(name.to_owned()),
            default_floor: floor,
            terrain: vec![floor],
        }
    }

    // A theme whose list holds a second terrain, so a draft can move its floor off the first.
    fn theme_def_with(
        key: ThemeUuid,
        name: &str,
        floor: TerrainUuid,
        second: TerrainUuid,
    ) -> UuidThemeDef {
        UuidThemeDef {
            key,
            display_name: ThemeDisplayName::new(name.to_owned()),
            default_floor: floor,
            terrain: vec![floor, second],
        }
    }

    #[test]
    fn a_sync_leaves_a_written_terrain_list_and_default_floor_alone() {
        let floor = terrain_key(4);
        let second = terrain_key(5);
        let added = terrain_key(6);
        let session = theme_key(4);
        let def = theme_def_with(session, "Industrial", floor, second);
        let registry = UuidThemeRegistry::new([(session, def.clone())]);

        let mut draft = ThemeDraft::default();
        load_theme_into_form(&mut draft, &def);
        draft.toggle_terrain(added);
        draft.set_default_floor(second);

        sync_theme_draft(EditorMode::Theme, session, &registry, &mut draft);

        assert!(
            draft.has_terrain(added),
            "the sync runs on every frame the Theme tab is drawn, so a terrain a write just \
             added must still be in the list one frame later: {:?}",
            draft.terrain(),
        );
        assert_eq!(
            draft.default_floor(),
            Some(second),
            "a default floor a write just moved must still be where the write put it, not back \
             on the registry def's own floor",
        );
    }

    #[test]
    fn new_theme_survives_sync_against_the_session_theme() {
        let slab = terrain_key(1);
        let session = theme_key(1);
        let registry = UuidThemeRegistry::new([(session, theme_def(session, "Hive", slab))]);

        let mut draft = ThemeDraft::new_theme();
        let minted = draft.key();
        sync_theme_draft(EditorMode::Theme, session, &registry, &mut draft);

        assert_eq!(
            draft.key(),
            minted,
            "sync must not replace a new_theme() draft with the session theme",
        );
        assert_eq!(
            draft.display_name(),
            "",
            "a new_theme() draft must stay blank after sync against a present session theme",
        );

        sync_theme_draft(EditorMode::Theme, session, &registry, &mut draft);
        assert_eq!(
            draft.key(),
            minted,
            "a later frame's sync against the same session must still leave the blank",
        );
    }

    #[test]
    fn switching_session_theme_loads_the_new_def() {
        let slab = terrain_key(2);
        let first = theme_key(2);
        let second = theme_key(3);
        let registry = UuidThemeRegistry::new([
            (first, theme_def(first, "First Theme", slab)),
            (second, theme_def(second, "Second Theme", slab)),
        ]);

        let mut draft = ThemeDraft::new_theme();
        sync_theme_draft(EditorMode::Theme, first, &registry, &mut draft);
        assert_eq!(
            draft.display_name(),
            "",
            "fixture: the first sync pins the blank against the first session theme",
        );

        sync_theme_draft(EditorMode::Theme, second, &registry, &mut draft);
        assert_eq!(
            draft.key(),
            second,
            "changing the session theme must load that entry even after New theme",
        );
        assert_eq!(
            draft.display_name(),
            "Second Theme",
            "the loaded def is the second registry entry, not the pinned blank",
        );

        let mut seedable = ThemeDraft::default();
        sync_theme_draft(EditorMode::Theme, first, &registry, &mut seedable);
        assert_eq!(
            seedable.key(),
            first,
            "Default stays seedable — first Theme tab visit still autoloads",
        );
    }
}
