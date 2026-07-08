//! The mode-panel **borrow context** — every model borrow the per-mode RIGHT-form and
//! CENTRAL-panel dispatches read, bundled once by the shell (GTW-670; the
//! `ModeSyncBundles` shape). Each `ResMut`-backed field keeps its OWN world lifetime —
//! a `&mut` is invariant over its type parameter, so one shared `'w` would force the
//! shell's independently elided param lifetimes to unify (a compile error — the GTW-669
//! lesson).
//!
//! The mutable model resources ride as `&mut ResMut<…>` (NOT pre-deref'd `&mut T`):
//! deref-mut fires Bevy's change detection, so pre-dereferencing every draft while
//! building the context would mark ALL of them changed EVERY frame regardless of the
//! active mode. Keeping the wrapper means the tick fires only inside the ACTIVE mode's
//! arm, exactly as it did when the dispatches lived inline in `shell.rs`.

use bevy::prelude::ResMut;
use gdtf_battle_sim::{
    level::UuidThemeRegistry, terrain::def::TerrainDefRegistry, weapon::WeaponRegistry,
};

use crate::{
    egui_shell::{
        params::{
            ArmorParams, AttachmentParams, GangParams, InjuryParams, PrefabParams, SpriteParams,
            WeaponParams,
        },
        textures::ResolvedTextures,
    },
    session::MapEditorSession,
    terrain_form::TerrainDraft,
    theme_form::ThemeDraft,
};

/// The per-mode model borrows the RIGHT-form + CENTRAL-panel dispatches thread —
/// borrowed from the shell's params for the duration of the two panel calls (GTW-670;
/// the module-layout seam continuation: the dispatch roster grows one arm per Workbench
/// mode, so the whole dispatch lives HERE and the shell only changes when the PANEL
/// LAYOUT does).
pub(in crate::egui_shell) struct ModePanelsCtx<
    'a,
    'sess,
    'ter,
    'theme,
    'prefab,
    'ps,
    'gang,
    'armor,
    'injury,
    'sprite,
    'ss,
    'attach,
    'weapon,
> {
    /// The shared authoring session (theme / grid / selected tile) — mutated by the
    /// PREFAB controls, read by the TERRAIN primary panel and the viewport.
    pub(in crate::egui_shell) session:          &'a mut ResMut<'sess, MapEditorSession>,
    /// The TERRAIN-mode authoring draft (the central primary panel's model).
    pub(in crate::egui_shell) terrain_draft:    &'a mut ResMut<'ter, TerrainDraft>,
    /// The THEME-mode authoring draft (the right form's + central library's model).
    pub(in crate::egui_shell) theme_draft:      &'a mut ResMut<'theme, ThemeDraft>,
    /// The PREFAB save-name text buffer (a shell `Local`).
    pub(in crate::egui_shell) prefab_save_name: &'a mut String,
    /// The UUID-keyed theme registry (option sources + viewport resolution) —
    /// pre-deref'd: a shared `Res` read carries no change-tick side effect.
    pub(in crate::egui_shell) themes:           Option<&'a UuidThemeRegistry>,
    /// The UUID-keyed terrain-def registry (pickers + the viewport's tile resolution).
    pub(in crate::egui_shell) terrain_registry: Option<&'a TerrainDefRegistry>,
    /// The ranged-weapons registry (the TERRAIN Emplacement mounted-weapon combo + the
    /// GANG loadout dropdowns).
    pub(in crate::egui_shell) weapons:          Option<&'a WeaponRegistry>,
    /// The pre-`ctx_mut` resolved egui texture ids the panels draw.
    pub(in crate::egui_shell) textures:         &'a ResolvedTextures,
    /// The PREFAB-mode model borrows (GTW-515).
    pub(in crate::egui_shell) prefab:           &'a mut PrefabParams<'prefab, 'ps>,
    /// The GANG-mode model borrows (GTW-636).
    pub(in crate::egui_shell) gang:             &'a mut GangParams<'gang>,
    /// The ARMOR-mode model borrows (GTW-479).
    pub(in crate::egui_shell) armor:            &'a mut ArmorParams<'armor>,
    /// The INJURY-mode model borrows (GTW-654).
    pub(in crate::egui_shell) injury:           &'a mut InjuryParams<'injury>,
    /// The SPRITE-mode model borrows (GTW-664).
    pub(in crate::egui_shell) sprite:           &'a mut SpriteParams<'sprite, 'ss>,
    /// The ATTACHMENT-mode model borrows (GTW-669).
    pub(in crate::egui_shell) attachment:       &'a mut AttachmentParams<'attach>,
    /// The WEAPON-mode model borrows (GTW-670).
    pub(in crate::egui_shell) weapon:           &'a mut WeaponParams<'weapon>,
}
