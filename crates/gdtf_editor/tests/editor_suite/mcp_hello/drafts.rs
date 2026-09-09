use bevy::app::App;
use gdtf_editor::{
    AttachmentDraft, GangDraft, InjuryDraft, MeleeWeaponDraft, SpriteDraft, TerrainDraft,
    ThemeDraft,
};

use crate::mcp_shared::support::TestError;

/// The gang draft the world holds right now.
pub(crate) fn gang_draft(app: &App) -> Result<GangDraft, TestError> {
    let Some(draft) = app.world().get_resource::<GangDraft>() else {
        return Err("the gang draft is a resource the editor creates on entering Editing".into());
    };
    Ok(draft.clone())
}

/// The injury draft the world holds right now.
pub(crate) fn injury_draft(app: &App) -> Result<InjuryDraft, TestError> {
    let Some(draft) = app.world().get_resource::<InjuryDraft>() else {
        return Err("the injury draft is a resource the editor creates on entering Editing".into());
    };
    Ok(draft.clone())
}

/// The sprite draft the world holds right now.
pub(crate) fn sprite_draft(app: &App) -> Result<SpriteDraft, TestError> {
    let Some(draft) = app.world().get_resource::<SpriteDraft>() else {
        return Err("the sprite draft is a resource the editor creates on entering Editing".into());
    };
    Ok(draft.clone())
}

/// The attachment draft the world holds right now.
pub(crate) fn attachment_draft(app: &App) -> Result<AttachmentDraft, TestError> {
    let Some(draft) = app.world().get_resource::<AttachmentDraft>() else {
        return Err(
            "the attachment draft is a resource the editor creates on entering Editing".into(),
        );
    };
    Ok(draft.clone())
}

/// The melee weapon draft the world holds right now.
pub(crate) fn melee_weapon_draft(app: &App) -> Result<MeleeWeaponDraft, TestError> {
    let Some(draft) = app.world().get_resource::<MeleeWeaponDraft>() else {
        return Err(
            "the melee weapon draft is a resource the editor creates on entering Editing".into(),
        );
    };
    Ok(draft.clone())
}

/// The terrain draft the world holds right now.
pub(crate) fn terrain_draft(app: &App) -> Result<TerrainDraft, TestError> {
    let Some(draft) = app.world().get_resource::<TerrainDraft>() else {
        return Err(
            "the terrain draft is a resource the editor creates on entering Editing".into(),
        );
    };
    Ok(draft.clone())
}

/// The theme draft the world holds right now.
pub(crate) fn theme_draft(app: &App) -> Result<ThemeDraft, TestError> {
    let Some(draft) = app.world().get_resource::<ThemeDraft>() else {
        return Err("the theme draft is a resource the editor creates on entering Editing".into());
    };
    Ok(draft.clone())
}
