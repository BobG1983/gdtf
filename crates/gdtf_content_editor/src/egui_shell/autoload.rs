use gdtf_assets::ContentSourcePaths;
use gdtf_battle_sim::{
    armor::ArmorRegistry,
    effects::fields::FieldDefRegistry,
    equipment::attachments::AttachmentRegistry,
    ganger::GangRegistry,
    injuries::{InjuryRegistry, InjuryTables},
    level::UuidThemeRegistry,
    weapon::{MeleeWeaponRegistry, WeaponRegistry},
};
use gdtf_content_families::{ThemeDefsFamily, sprites::SpriteDefRegistry};

use crate::{
    armor_form::ArmorDraft,
    attachment_form::AttachmentDraft,
    egui_shell::{
        armor_form_ui, attachment_form_ui, field_form_ui, gang_form_ui, injury_form_ui,
        melee_weapon_form_ui,
        params::{
            ArmorParams, AttachmentParams, FieldParams, GangParams, InjuryParams,
            MeleeWeaponParams, SpriteParams, WeaponParams,
        },
        sprite_form_ui, theme_form_ui, weapon_form_ui,
    },
    field_form::FieldDraft,
    gang_form::GangDraft,
    injury_form::{InjuryDraft, WeightingDraft},
    melee_weapon_form::MeleeWeaponDraft,
    mode::EditorMode,
    session::MapEditorSession,
    sprite_form::SpriteDraft,
    theme_form::ThemeDraft,
    weapon_form::WeaponDraft,
};

pub(super) struct ModeSyncBundles<
    'a,
    'gang,
    'armor,
    'injury,
    'sprite,
    'attach,
    'weapon,
    'melee,
    'field,
    's,
> {
    pub(super) gang:         &'a mut GangParams<'gang>,
    pub(super) armor:        &'a mut ArmorParams<'armor>,
    pub(super) injury:       &'a mut InjuryParams<'injury>,
    pub(super) sprite:       &'a mut SpriteParams<'sprite, 's>,
    pub(super) attachment:   &'a mut AttachmentParams<'attach>,
    pub(super) weapon:       &'a mut WeaponParams<'weapon>,
    pub(super) melee_weapon: &'a mut MeleeWeaponParams<'melee>,
    pub(super) field:        &'a mut FieldParams<'field>,
}

pub(super) fn run_form_syncs(
    mode: EditorMode,
    session: &MapEditorSession,
    themes: Option<&UuidThemeRegistry>,
    theme_draft: &mut ThemeDraft,
    theme_sources: Option<&ContentSourcePaths<ThemeDefsFamily>>,
    bundles: ModeSyncBundles<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>,
) {
    theme_form_sync(mode, session, themes, theme_draft, theme_sources);
    gang_form_sync(
        mode,
        bundles.gang.draft.as_deref_mut(),
        bundles.gang.gangs.as_deref(),
    );
    armor_form_sync(
        mode,
        bundles.armor.draft.as_deref_mut(),
        bundles.armor.registry.as_deref(),
    );
    injury_form_sync(
        mode,
        bundles.injury.draft.as_deref_mut(),
        bundles.injury.registry.as_deref(),
        bundles.injury.weighting.as_deref_mut(),
        bundles.injury.tables.as_deref(),
    );
    sprite_form_sync(
        mode,
        bundles.sprite.draft.as_deref_mut(),
        bundles.sprite.registry.as_deref(),
    );
    attachment_form_sync(
        mode,
        bundles.attachment.draft.as_deref_mut(),
        bundles.attachment.registry.as_deref(),
    );
    weapon_form_sync(
        mode,
        bundles.weapon.draft.as_deref_mut(),
        bundles.weapon.registry.as_deref(),
    );
    melee_weapon_form_sync(
        mode,
        bundles.melee_weapon.draft.as_deref_mut(),
        bundles.melee_weapon.registry.as_deref(),
    );
    field_form_sync(
        mode,
        bundles.field.draft.as_deref_mut(),
        bundles.field.registry.as_deref(),
    );
}

fn theme_form_sync(
    mode: EditorMode,
    session: &MapEditorSession,
    themes: Option<&UuidThemeRegistry>,
    draft: &mut ThemeDraft,
    sources: Option<&ContentSourcePaths<ThemeDefsFamily>>,
) {
    let Some(themes) = themes else {
        return;
    };
    theme_form_ui::sync_theme_draft(mode, session.theme(), themes, draft, sources);
}

fn gang_form_sync(mode: EditorMode, draft: Option<&mut GangDraft>, gangs: Option<&GangRegistry>) {
    if mode == EditorMode::Gang
        && let (Some(draft), Some(registry)) = (draft, gangs)
    {
        gang_form_ui::autoload_first_gang(draft, registry);
    }
}

fn armor_form_sync(
    mode: EditorMode,
    draft: Option<&mut ArmorDraft>,
    registry: Option<&ArmorRegistry>,
) {
    if mode == EditorMode::Armor
        && let (Some(draft), Some(registry)) = (draft, registry)
    {
        armor_form_ui::autoload_first_armor(draft, registry);
    }
}

fn injury_form_sync(
    mode: EditorMode,
    draft: Option<&mut InjuryDraft>,
    registry: Option<&InjuryRegistry>,
    weighting: Option<&mut WeightingDraft>,
    tables: Option<&InjuryTables>,
) {
    if mode != EditorMode::Injury {
        return;
    }
    if let (Some(draft), Some(registry)) = (draft, registry) {
        injury_form_ui::autoload_first_injury(draft, registry);
    }
    if let (Some(weighting), Some(tables)) = (weighting, tables) {
        injury_form_ui::autoload_weighting_table(weighting, tables);
    }
}

fn sprite_form_sync(
    mode: EditorMode,
    draft: Option<&mut SpriteDraft>,
    registry: Option<&SpriteDefRegistry>,
) {
    if mode == EditorMode::Sprite
        && let (Some(draft), Some(registry)) = (draft, registry)
    {
        sprite_form_ui::autoload_first_sprite(draft, registry);
    }
}

fn attachment_form_sync(
    mode: EditorMode,
    draft: Option<&mut AttachmentDraft>,
    registry: Option<&AttachmentRegistry>,
) {
    if mode == EditorMode::Attachment
        && let (Some(draft), Some(registry)) = (draft, registry)
    {
        attachment_form_ui::autoload_first_attachment(draft, registry);
    }
}

fn weapon_form_sync(
    mode: EditorMode,
    draft: Option<&mut WeaponDraft>,
    registry: Option<&WeaponRegistry>,
) {
    if mode == EditorMode::Weapon
        && let (Some(draft), Some(registry)) = (draft, registry)
    {
        weapon_form_ui::autoload_first_weapon(draft, registry);
    }
}

fn melee_weapon_form_sync(
    mode: EditorMode,
    draft: Option<&mut MeleeWeaponDraft>,
    registry: Option<&MeleeWeaponRegistry>,
) {
    if mode == EditorMode::MeleeWeapon
        && let (Some(draft), Some(registry)) = (draft, registry)
    {
        melee_weapon_form_ui::autoload_first_melee_weapon(draft, registry);
    }
}

fn field_form_sync(
    mode: EditorMode,
    draft: Option<&mut FieldDraft>,
    registry: Option<&FieldDefRegistry>,
) {
    if mode == EditorMode::Field
        && let (Some(draft), Some(registry)) = (draft, registry)
    {
        field_form_ui::autoload_first_field(draft, registry);
    }
}
