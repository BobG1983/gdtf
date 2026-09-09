use bevy::app::App;
use gdtf_battle_sim::{
    equipment::attachments::AttachmentRegistry, ganger::GangRegistry, injuries::InjuryRegistry,
    level::UuidThemeRegistry, terrain::def::TerrainDefRegistry, weapon::MeleeWeaponRegistry,
};
use gdtf_content_families::sprites::SpriteDefRegistry;

use crate::mcp_shared::support::TestError;

// The first key in sorted order, so no case pins a content file name.
fn first_sorted(mut keys: Vec<String>, family: &str) -> Result<String, TestError> {
    keys.sort();
    let Some(first) = keys.first() else {
        return Err(
            format!("the loaded {family} registry is empty, so no key can be loaded").into(),
        );
    };
    Ok(first.clone())
}

/// A key the live gang registry actually holds.
pub(crate) fn first_gang_key(app: &App) -> Result<String, TestError> {
    let Some(registry) = app.world().get_resource::<GangRegistry>() else {
        return Err("the editor reached Editing, so its gang registry is loaded".into());
    };
    first_sorted(
        registry.keys().map(|key| key.as_str().to_owned()).collect(),
        "gang",
    )
}

/// A key the live injury registry actually holds.
pub(crate) fn first_injury_key(app: &App) -> Result<String, TestError> {
    let Some(registry) = app.world().get_resource::<InjuryRegistry>() else {
        return Err("the editor reached Editing, so its injury registry is loaded".into());
    };
    first_sorted(
        registry
            .iter()
            .map(|(key, _)| key.as_str().to_owned())
            .collect(),
        "injury",
    )
}

/// A key the live sprite registry actually holds.
pub(crate) fn first_sprite_key(app: &App) -> Result<String, TestError> {
    let Some(registry) = app.world().get_resource::<SpriteDefRegistry>() else {
        return Err("the editor reached Editing, so its sprite registry is loaded".into());
    };
    first_sorted(
        registry.keys().map(|key| key.as_str().to_owned()).collect(),
        "sprite",
    )
}

/// A key the live attachment registry actually holds.
pub(crate) fn first_attachment_key(app: &App) -> Result<String, TestError> {
    let Some(registry) = app.world().get_resource::<AttachmentRegistry>() else {
        return Err("the editor reached Editing, so its attachment registry is loaded".into());
    };
    first_sorted(
        registry.keys().map(|key| key.as_str().to_owned()).collect(),
        "attachment",
    )
}

/// A key the live melee weapon registry actually holds.
pub(crate) fn first_melee_weapon_key(app: &App) -> Result<String, TestError> {
    let Some(registry) = app.world().get_resource::<MeleeWeaponRegistry>() else {
        return Err("the editor reached Editing, so its melee weapon registry is loaded".into());
    };
    first_sorted(
        registry.keys().map(|key| key.as_str().to_owned()).collect(),
        "melee weapon",
    )
}

/// A key the live terrain registry actually holds, rendered the way the host renders it.
pub(crate) fn first_terrain_key(app: &App) -> Result<String, TestError> {
    let Some(registry) = app.world().get_resource::<TerrainDefRegistry>() else {
        return Err("the editor reached Editing, so its terrain registry is loaded".into());
    };
    first_sorted(
        registry
            .defs()
            .map(|(key, _)| (**key).to_string())
            .collect(),
        "terrain",
    )
}

/// A key the live theme registry actually holds, rendered the way the host renders it.
pub(crate) fn first_theme_key(app: &App) -> Result<String, TestError> {
    let Some(registry) = app.world().get_resource::<UuidThemeRegistry>() else {
        return Err("the editor reached Editing, so its theme registry is loaded".into());
    };
    first_sorted(
        registry
            .defs()
            .map(|(key, _)| (**key).to_string())
            .collect(),
        "theme",
    )
}
