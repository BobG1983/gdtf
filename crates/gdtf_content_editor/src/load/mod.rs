//! The editor's `Load` pass: register the SAME generic asset seams the game
//! registers, then transition to [`Editing`](crate::EditorState::Editing) once
//! every resolved resource exists.
//!
//! Wiring-only module: [`register_load`] does the registration; the transition
//! gate lives in [`transition`] and the editor-owned tile-role fallback in
//! [`fallback`].
//!
//! # One source, two hosts (GTW-579)
//!
//! The editor is a SECOND in-app asset host beside the game, but it no longer
//! re-implements the game's Load pass: every asset it hosts loads through the
//! generic seams in `gdtf_assets`, with the SAME definitions the game
//! registers, so an authored file resolves IDENTICALLY in game and editor:
//!
//! - The four FOLDER families — ranged weapons, armor, the UUID-keyed terrain
//!   defs + theme defs — register through the GTW-570
//!   [`register_content_family`](gdtf_assets::ContentFamilyAppExt) seam using
//!   the SAME `gdtf_content_families` glue impls the game registers. Each
//!   family's registry-build logic therefore has exactly ONE definition
//!   workspace-wide (the seam's shared folder walk).
//! - The two SINGLE-ASSET chains — the [`GdtfTheme`](gdtf_ui::theme::GdtfTheme)
//!   and the presenter [`TileRoles`](gdtf_battle_presenter::TileRoles) table —
//!   install the GTW-564 generic hot-RON chain from the chain OWNERS' published
//!   configs ([`theme_hot_ron_chain`] / [`tile_roles_hot_ron_chain`], which
//!   single-source each path + map hook), re-configured with the editor's
//!   ADR-0003 fallback (see below).
//!
//! **Adding an editor-consumed family** costs at most two edits: ONE
//! `register_content_family::<F>()` line in [`register_load`], plus a
//! `ContentFamily` glue impl in `gdtf_content_families` ONLY if the game does
//! not already define the family.
//!
//! # Editor-specific load policy (stays editor-owned — GTW-579 C4)
//!
//! - **Whole-session handle persistence (GTW-533):** the seam's persistent
//!   [`ContentFolderHandle`](gdtf_assets::ContentFolderHandle) /
//!   [`HotRonHandle`](gdtf_assets::HotRonHandle) resources are inserted at
//!   `Startup` and NEVER removed — `register_load` registers no
//!   `OnExit(EditorState::Load)` cleanup — so a live `.ron` edit re-enumerates
//!   folder members and refreshes the resolved resources with NO restart (the
//!   editor half of the "hot-reload in-app, game AND editor" contract, through
//!   the ONE shared Bevy `file_watcher` mechanism).
//! - **ADR-0003 fail-safe:** a `Failed` asset falls back to a const default so
//!   the editor never hangs in `Load` — the folder families fail closed to the
//!   seam's EMPTY registry; the theme falls back to the published
//!   [`default_theme`](gdtf_ui::theme::default_theme) and the tile roles to the
//!   editor-owned zero table ([`fallback`]), each attached HERE via
//!   [`HotRonChain::with_fallback`](gdtf_assets::HotRonChain::with_fallback)
//!   (the game's registrations of the same chains stay fallback-less — the
//!   policy rides the host's registration, never a seam mode flag).
//! - **Headless inertness:** every seam ext call self-gates on an
//!   [`AssetServer`](bevy::asset::AssetServer) being present (`bevy-traps.md`
//!   #1), so a `MinimalPlugins` harness registers no loaders and no systems.
//! - **Own-absence gating (`bevy-traps.md` #3):** each generic resolve is
//!   registered `run_if(handle-present AND not(resource_exists::<Registry>))`,
//!   so every branch gates on its OWN resource's absence and none starves
//!   another.
//! - **The transition is unchanged:** [`transition_to_editing`] still fires
//!   only when ALL six resolved resources exist.

mod fallback;
mod transition;

use bevy::prelude::*;
use gdtf_assets::{ContentFamilyAppExt, HotRonAppExt};
use gdtf_battle_presenter::tile_roles_hot_ron_chain;
use gdtf_content_families::{ArmorFamily, TerrainDefsFamily, ThemeDefsFamily, WeaponsFamily};
use gdtf_ui::{theme::default_theme, theme_hot_ron_chain};

use crate::{
    EditorState,
    load::{fallback::default_tile_roles, transition::transition_to_editing},
};

/// Registers the editor's `Load` asset pass onto `app` — six seam registrations
/// plus the transition gate (see the [module docs](self) for the seam-vs-policy
/// split).
///
/// Each ext call wires its family's/chain's WHOLE generic kick-off (`Startup`,
/// storing the persistent handle), gated resolve (inserts the resource exactly
/// once — or its fallback/empty default on a genuine `Failed`), and ungated
/// live redrive (the GTW-533 hot-reload, which keeps firing AFTER `Load` exits
/// because the handles persist). The transition runs in `Update` while `Load`
/// and leaves for [`Editing`](EditorState::Editing) once every resolved
/// resource exists — reached even on an all-failed asset root (the no-strand
/// guarantee).
pub(crate) fn register_load(app: &mut App) {
    // The two single-asset chains (GTW-564 seam), installed from the chain
    // owners' PUBLISHED configs — path + map hook stay single-sourced in
    // gdtf_ui / gdtf_battle_presenter — with the editor's ADR-0003 fallback
    // attached at THIS registration (editor-owned policy, GTW-579 C4b).
    app.init_hot_ron_chain(theme_hot_ron_chain().with_fallback(default_theme));
    app.init_hot_ron_chain(tile_roles_hot_ron_chain().with_fallback(default_tile_roles));

    // The four folder families (GTW-570 seam) — the SAME glue-crate family
    // definitions the game registers, so game and editor build each registry
    // through literally one function. The terrain + theme defs share the ONE
    // MIXED `content/terrain/` tree; the seam's unconditional TypeId filter
    // keeps each walk to its own members.
    app.register_content_family::<WeaponsFamily>();
    app.register_content_family::<ArmorFamily>();
    app.register_content_family::<TerrainDefsFamily>();
    app.register_content_family::<ThemeDefsFamily>();

    app.add_systems(
        Update,
        transition_to_editing.run_if(in_state(EditorState::Load)),
    );
}
