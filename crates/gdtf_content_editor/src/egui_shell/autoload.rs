//! The shell's PRE-PANEL per-mode model-sync / autoload block — split out of
//! `shell.rs` at the GTW-479-flagged natural boundary (GTW-654; module-layout warn
//! band): these runners change when a MODE's open-with-content behavior does, the
//! shell when the PANEL layout does. Each runner self-gates on its mode and on its
//! model borrows being present, and each is idempotent under the egui multipass
//! re-run (bevy-traps #8) — the underlying one-shot seeds end their own pending
//! state on the first pass.

use gdtf_battle_sim::{
    armor::ArmorRegistry,
    equipment::attachments::AttachmentRegistry,
    ganger::GangRegistry,
    injuries::{InjuryRegistry, InjuryTables},
    level::UuidThemeRegistry,
    weapon::{MeleeWeaponRegistry, WeaponRegistry},
};
use gdtf_content_families::sprites::SpriteDefRegistry;

use crate::{
    armor_form::ArmorDraft,
    attachment_form::AttachmentDraft,
    egui_shell::{
        armor_form_ui, attachment_form_ui, gang_form_ui, injury_form_ui, melee_weapon_form_ui,
        params::{
            ArmorParams, AttachmentParams, GangParams, InjuryParams, MeleeWeaponParams,
            SpriteParams, WeaponParams,
        },
        sprite_form_ui, theme_form_ui, weapon_form_ui,
    },
    gang_form::GangDraft,
    injury_form::{InjuryDraft, WeightingDraft},
    melee_weapon_form::MeleeWeaponDraft,
    mode::EditorMode,
    session::MapEditorSession,
    sprite_form::SpriteDraft,
    theme_form::ThemeDraft,
    weapon_form::WeaponDraft,
};

/// The per-mode model bundles the sync fan-out below reads — borrowed from the shell's
/// `SystemParam` bundles for the duration of [`run_form_syncs`] (GTW-669; the
/// continuing the module-layout split: the sync ROSTER grows with every new mode, so the
/// whole fan-out lives HERE with the sync runners, and the shell only changes when the
/// PANEL layout does). Each bundle keeps its OWN world lifetime — a `&mut` is invariant
/// over its type parameter, so one shared `'w` would force the shell's independently
/// elided param lifetimes to unify (a compile error).
pub(super) struct ModeSyncBundles<'a, 'gang, 'armor, 'injury, 'sprite, 'attach, 'weapon, 'melee, 's>
{
    /// The GANG-mode model borrows (GTW-636).
    pub(super) gang:         &'a mut GangParams<'gang>,
    /// The ARMOR-mode model borrows (GTW-479).
    pub(super) armor:        &'a mut ArmorParams<'armor>,
    /// The INJURY-mode model borrows (GTW-654).
    pub(super) injury:       &'a mut InjuryParams<'injury>,
    /// The SPRITE-mode model borrows (GTW-664).
    pub(super) sprite:       &'a mut SpriteParams<'sprite, 's>,
    /// The ATTACHMENT-mode model borrows (GTW-669).
    pub(super) attachment:   &'a mut AttachmentParams<'attach>,
    /// The WEAPON-mode model borrows (GTW-670).
    pub(super) weapon:       &'a mut WeaponParams<'weapon>,
    /// The MELEE-WEAPON-mode model borrows (GTW-671).
    pub(super) melee_weapon: &'a mut MeleeWeaponParams<'melee>,
}

/// Run EVERY per-mode pre-panel sync/autoload runner — the ONE fan-out the shell calls
/// before resolving textures / declaring panels (GTW-669). Each runner self-gates on
/// its mode and its borrows being present, and each is idempotent under the egui
/// multipass re-run (bevy-traps #8), so running the whole roster every frame is safe.
pub(super) fn run_form_syncs(
    mode: EditorMode,
    session: &MapEditorSession,
    themes: Option<&UuidThemeRegistry>,
    theme_draft: &mut ThemeDraft,
    bundles: ModeSyncBundles<'_, '_, '_, '_, '_, '_, '_, '_, '_>,
) {
    theme_form_sync(mode, session, themes, theme_draft);
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
}

/// C3.2 (GTW-514): when in THEME mode with a theme already selected in the session,
/// auto-load that theme's def into the form so the author edits the live
/// definition. Checked every frame; `resolve_autoload` returns `None` for a nil
/// theme / absent registry, so it no-ops until a real theme resolves. The key
/// comparison avoids redundant reinitialisation across frames — it only loads when
/// the form's current key differs from the session theme (a new selection or a
/// first-enter with a pre-selected theme).
fn theme_form_sync(
    mode: EditorMode,
    session: &MapEditorSession,
    themes: Option<&UuidThemeRegistry>,
    draft: &mut ThemeDraft,
) {
    if mode == EditorMode::Theme
        && let Some(themes) = themes
        && let Some(def) = theme_form_ui::resolve_autoload(session.theme(), themes)
        && draft.key() != def.key
    {
        theme_form_ui::load_theme_into_form(draft, def);
    }
}

/// GTW-636: the GANG mode's one-shot open-with-a-gang seed — a still-pristine
/// draft loads the FIRST gang (sorted) from the resolved registry, the retired
/// in-game editor's exact open behavior.
fn gang_form_sync(mode: EditorMode, draft: Option<&mut GangDraft>, gangs: Option<&GangRegistry>) {
    if mode == EditorMode::Gang
        && let (Some(draft), Some(registry)) = (draft, gangs)
    {
        gang_form_ui::autoload_first_gang(draft, registry);
    }
}

/// GTW-479: the ARMOR mode's one-shot open-with-an-armor seed — the Gang
/// autoload's exact parity twin.
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

/// GTW-654: the INJURY mode's TWO one-shot seeds — the def form opens on the first
/// sorted injury (the Gang/Armor parity) and the weighting section on the first
/// canonical category's current built table.
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

/// GTW-664: the SPRITE mode's one-shot open-with-a-sprite seed — the Gang / Armor
/// autoloads' exact parity twin over the GTW-663 [`SpriteDefRegistry`].
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

/// GTW-669: the ATTACHMENT mode's one-shot open-with-an-item seed — the Gang / Armor /
/// Sprite autoloads' exact parity twin over the GTW-619 [`AttachmentRegistry`].
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

/// GTW-670: the WEAPON mode's one-shot open-with-a-weapon seed — the Gang / Armor /
/// Attachment autoloads' exact parity twin over the GTW-257 [`WeaponRegistry`].
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

/// GTW-671: the MELEE-WEAPON mode's one-shot open-with-a-weapon seed — the Weapon
/// autoload's exact parity twin over the GTW-505 [`MeleeWeaponRegistry`].
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
