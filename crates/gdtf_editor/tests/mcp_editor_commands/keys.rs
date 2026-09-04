use bevy::{app::App, asset::uuid::Uuid};
use gdtf_battle_sim::{
    level::ThemeUuid,
    terrain::{
        def::{TerrainDefRegistry, TerrainUuid},
        entity::TerrainPieceKind,
    },
};

use crate::{
    drafts::theme_draft,
    names::EDITOR_FAMILIES,
    outcome::ran_body,
    rows::{FamiliesReplyRow, FamilyRow},
    socket::{Client, run_editor},
    support::TestError,
    world::{session, terrain_registry, theme_registry},
};

/// Text no UUID parser accepts, so a helper refuses it before it reaches any registry.
pub(crate) const A_KEY_THAT_IS_NOT_UUID_TEXT: &str = "not-a-uuid";

/// The arguments every one of the three helpers takes.
pub(crate) fn key_args(key: &str) -> String {
    format!("(key: \"{key}\")")
}

// The floor picker offers slabs and nothing else, whatever else the draft holds.
fn is_a_slab(registry: &TerrainDefRegistry, key: TerrainUuid) -> bool {
    registry
        .def(&key)
        .is_some_and(|def| def.sim_kind.kind() == TerrainPieceKind::Slab)
}

// The host's own list of theme keys, so no case pins a UUID or a content file name.
fn theme_keys(app: &mut App, client: &mut Client) -> Result<Vec<String>, TestError> {
    let reply = client.exchange(app, &run_editor(EDITOR_FAMILIES, "(family: Some(Theme))"))?;
    let body: FamiliesReplyRow = ran_body(&reply, EDITOR_FAMILIES)?;
    Ok(body
        .families
        .into_iter()
        .flat_map(|family| family.entries)
        .map(|entry| entry.key)
        .collect())
}

#[test]
fn an_unfiltered_families_read_carries_the_field_family_with_the_registrys_own_keys()
-> Result<(), TestError> {
    let (mut app, mut client) = crate::harness::editing_app_and_client()?;
    let reply = client.exchange(&mut app, &run_editor(EDITOR_FAMILIES, "()"))?;
    let body: FamiliesReplyRow = ran_body(&reply, EDITOR_FAMILIES)?;

    let Some(fields) = body
        .families
        .iter()
        .find(|family| family.family == FamilyRow::Field)
    else {
        return Err("an unfiltered families read answers every family, Field among them".into());
    };
    let mut reported: Vec<String> = fields
        .entries
        .iter()
        .map(|entry| entry.key.clone())
        .collect();
    reported.sort();
    assert!(
        !reported.is_empty(),
        "the Field family reads the loaded FieldDefRegistry, so an arm that answers an empty \
         list leaves the right rail's picker with nothing the wire can see",
    );

    let mut held: Vec<String> = field_registry(&app)?;
    held.sort();
    assert_eq!(
        reported, held,
        "the reply's entries are the registry's own keys",
    );
    Ok(())
}

// The keys the loaded field registry holds, so no case pins a content file name.
fn field_registry(app: &App) -> Result<Vec<String>, TestError> {
    let Some(registry) = app
        .world()
        .get_resource::<gdtf_battle_sim::effects::fields::FieldDefRegistry>()
    else {
        return Err("the editor reached Editing, so its field registry is loaded".into());
    };
    Ok(registry.keys().map(|key| key.as_str().to_owned()).collect())
}

/// A theme key the host reports that the session is not already on.
pub(crate) fn a_theme_off_the_session(
    app: &mut App,
    client: &mut Client,
) -> Result<String, TestError> {
    let held = (*session(app)?.theme()).to_string();
    let keys = theme_keys(app, client)?;
    let Some(other) = keys.iter().find(|key| **key != held) else {
        return Err(format!(
            "this case needs more than one theme loaded, and the host reports {keys:?}"
        )
        .into());
    };
    Ok(other.clone())
}

/// A key the theme registry holds no theme under, taken from another family's live keys.
pub(crate) fn a_key_no_theme_holds(app: &App) -> Result<String, TestError> {
    let themes = theme_registry(app)?;
    let mut keys: Vec<TerrainUuid> = terrain_registry(app)?.defs().map(|(key, _)| *key).collect();
    keys.sort_by_key(|key| (**key).to_string());
    let Some(key) = keys
        .into_iter()
        .find(|key| themes.def(&ThemeUuid::new(**key)).is_none())
    else {
        return Err(
            "every terrain key is also a theme key, so this case has no key the theme \
                    registry refuses"
                .into(),
        );
    };
    Ok((*key).to_string())
}

/// A key the terrain registry holds no def under, taken from another family's live keys.
pub(crate) fn a_key_no_terrain_holds(
    app: &mut App,
    client: &mut Client,
) -> Result<String, TestError> {
    let terrain = terrain_registry(app)?;
    let keys = theme_keys(app, client)?;
    let Some(key) = keys.iter().find(|key| {
        Uuid::parse_str(key).is_ok_and(|parsed| terrain.def(&TerrainUuid::new(parsed)).is_none())
    }) else {
        return Err(format!(
            "this case needs a key the terrain registry does not hold, and every theme key the \
             host reports is also a terrain key: {keys:?}"
        )
        .into());
    };
    Ok(key.clone())
}

/// A terrain the registry holds that the loaded draft does not.
pub(crate) fn a_terrain_outside_the_draft(app: &App) -> Result<TerrainUuid, TestError> {
    let draft = theme_draft(app)?;
    let mut outside: Vec<TerrainUuid> = terrain_registry(app)?
        .defs()
        .map(|(key, _)| *key)
        .filter(|key| !draft.has_terrain(*key))
        .collect();
    outside.sort_by_key(|key| (**key).to_string());
    let Some(first) = outside.first() else {
        return Err("this case needs a terrain the loaded theme does not already hold".into());
    };
    Ok(*first)
}

/// A slab the loaded draft holds that is not already its default floor.
pub(crate) fn a_slab_in_the_draft(app: &App) -> Result<TerrainUuid, TestError> {
    let registry = terrain_registry(app)?;
    let draft = theme_draft(app)?;
    let floor = draft.default_floor();
    let Some(key) = draft
        .terrain()
        .iter()
        .copied()
        .find(|key| Some(*key) != floor && is_a_slab(&registry, *key))
    else {
        return Err(
            "this case needs the loaded theme to hold a slab other than its own default \
                    floor, or nothing here could move the floor"
                .into(),
        );
    };
    Ok(key)
}

/// A terrain the loaded draft holds that is not a slab, so the floor picker never offers it.
pub(crate) fn a_non_slab_in_the_draft(app: &App) -> Result<TerrainUuid, TestError> {
    let registry = terrain_registry(app)?;
    let draft = theme_draft(app)?;
    let Some(key) = draft
        .terrain()
        .iter()
        .copied()
        .find(|key| !is_a_slab(&registry, *key))
    else {
        return Err("this case needs the loaded theme to hold a terrain that is not a slab".into());
    };
    Ok(key)
}
