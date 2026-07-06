//! The `Load`-pass registrar — [`register_load`], the six generic seam registrations plus the
//! transition gate (extracted from the module wiring so `mod.rs` stays fn-free).

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
/// plus the transition gate (see the [module docs](super) for the seam-vs-policy
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
