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
//! - The SINGLE-ASSET chain — the presenter
//!   [`TileRoles`](gdtf_battle_presenter::TileRoles) table — installs the
//!   GTW-564 generic hot-RON chain from the chain OWNER's published config
//!   ([`tile_roles_hot_ron_chain`](gdtf_battle_presenter::tile_roles_hot_ron_chain),
//!   which single-sources the path + map hook), re-configured with the editor's
//!   ADR-0003 fallback (see below). The game's `GdtfTheme` chain is NOT
//!   installed: the egui shell styles itself, so the editor reads no theme
//!   field (GTW-625 — the GTW-579 AC2 amendment).
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
//!   seam's EMPTY registry; the tile roles fall back to the
//!   editor-owned zero table ([`fallback`]), attached HERE via
//!   [`HotRonChain::with_fallback`](gdtf_assets::HotRonChain::with_fallback)
//!   (the game's registration of the same chain stays fallback-less — the
//!   policy rides the host's registration, never a seam mode flag).
//! - **Headless inertness:** every seam ext call self-gates on an
//!   [`AssetServer`](bevy::asset::AssetServer) being present (`bevy-traps.md`
//!   #1), so a `MinimalPlugins` harness registers no loaders and no systems.
//! - **Own-absence gating (`bevy-traps.md` #3):** each generic resolve is
//!   registered `run_if(handle-present AND not(resource_exists::<Registry>))`,
//!   so every branch gates on its OWN resource's absence and none starves
//!   another.
//! - **The transition gates on every resolved resource:** [`transition_to_editing`](transition::transition_to_editing) fires
//!   only when ALL FIVE resolved resources exist (the four folder registries +
//!   the tile-role table; the game theme is not among them — GTW-625).

mod fallback;
mod register;
mod transition;

pub(crate) use register::register_load;
