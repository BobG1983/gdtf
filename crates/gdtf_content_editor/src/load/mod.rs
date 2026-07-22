//! The editor's `Load` pass: register the SAME generic content-family
//! registrations the game registers, then transition to
//! [`Editing`](crate::EditorState::Editing) once every resolved resource exists.
//!
//! Wiring-only module: [`register_load`] does the registration; the transition
//! gate lives in [`transition`], and the bespoke injuries pass (GTW-654 — one
//! folder, two resources, off the generic registration helper by design) in
//! [`injuries`].
//!
//! # One source, two hosts (GTW-579)
//!
//! The editor is a SECOND in-app asset host beside the game, but it no longer
//! re-implements the game's Load pass: every asset it hosts loads through the
//! generic registration helpers in `gdtf_assets`, with the SAME definitions the
//! game registers, so an authored file resolves IDENTICALLY in game and editor:
//!
//! - The eight FOLDER families — ranged weapons, armor, the UUID-keyed terrain
//!   defs + theme defs, the gangs + melee weapons the GANG mode edits
//!   (GTW-636), the sprite defs every terrain graphic resolves through
//!   (GTW-663/665), plus the attachments the ATTACHMENT mode edits
//!   (GTW-669) — register through the GTW-570
//!   [`register_content_family`](gdtf_assets::ContentFamilyAppExt) helper using
//!   the SAME `gdtf_content_families` glue impls the game registers. Each
//!   family's registry-build logic therefore has exactly ONE definition
//!   workspace-wide (the helper's shared folder walk).
//! - The BESPOKE injuries family (GTW-654) — one folder resolving into TWO
//!   resources ([`InjuryRegistry`](gdtf_battle_sim::injuries::InjuryRegistry) +
//!   [`InjuryTables`](gdtf_battle_sim::injuries::InjuryTables)), a declared
//!   exclusion from the GTW-570 registration helper — registers through its own thin pass
//!   ([`injuries`]), whose folder walk + salvage are the SAME
//!   `gdtf_content_families::injuries` halves the game's Load resolve runs.
//! - NO single-asset hot-RON chain remains: GTW-665 retired the presenter's
//!   tile-role chain (terrain graphics resolve through the sprite-defs FAMILY
//!   above), and the game's `GdtfTheme` chain was never installed — the egui
//!   shell styles itself, so the editor reads no theme field (GTW-625 — the
//!   GTW-579 AC2 amendment).
//!
//! **Adding an editor-consumed family** costs at most two edits: ONE
//! `register_content_family::<F>()` line in [`register_load`], plus a
//! `ContentFamily` glue impl in `gdtf_content_families` ONLY if the game does
//! not already define the family.
//!
//! # Editor-specific load policy (stays editor-owned — GTW-579 C4)
//!
//! - **Whole-session handle persistence (GTW-533):** the registration helper's persistent
//!   [`ContentFolderHandle`](gdtf_assets::ContentFolderHandle) resources are
//!   inserted at `Startup` and NEVER removed — `register_load` registers no
//!   `OnExit(EditorState::Load)` cleanup — so a live `.ron` edit re-enumerates
//!   folder members and refreshes the resolved resources with NO restart (the
//!   editor half of the "hot-reload in-app, game AND editor" contract, through
//!   the ONE shared Bevy `file_watcher` mechanism).
//! - **ADR-0003 fail-safe:** a `Failed` asset falls back to a const default so
//!   the editor never hangs in `Load` — the folder families (every editor-gated
//!   asset since GTW-665 retired the tile-role chain) fail closed to the
//!   registration helper's EMPTY registry.
//! - **Headless fallback:** every registration ext call self-gates on an
//!   [`AssetServer`](bevy::asset::AssetServer) being present (`bevy-traps.md`
//!   #1), so a `MinimalPlugins` harness registers no loaders and no systems —
//!   each family registration instead seeds its DEFAULT registry (the GTW-629
//!   rider), so the presence-gated transition still releases headless.
//! - **Own-absence gating (`bevy-traps.md` #3):** each generic resolve is
//!   registered `run_if(handle-present AND not(resource_exists::<Registry>))`,
//!   so every branch gates on its OWN resource's absence and none starves
//!   another.
//! - **The transition gates on every resolved resource:** [`transition_to_editing`](transition::transition_to_editing) fires
//!   only when ALL TEN resolved resources exist (the eight folder registries +
//!   the injuries pair; the game theme is not among them — GTW-625).

mod injuries;
mod register;
mod transition;

pub(crate) use register::register_load;
