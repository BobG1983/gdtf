//! The `Load`-pass registrar — [`register_load`], the eight generic seam registrations plus the
//! transition gate (extracted from the module wiring so `mod.rs` stays fn-free).

use bevy::prelude::*;
use gdtf_assets::{ContentFamilyAppExt, HotRonAppExt};
use gdtf_battle_presenter::tile_roles_hot_ron_chain;
use gdtf_content_families::{
    ArmorFamily, GangsFamily, MeleeWeaponsFamily, SpriteDefsFamily, TerrainDefsFamily,
    ThemeDefsFamily, WeaponsFamily,
};

use crate::{
    EditorState,
    load::{
        fallback::default_tile_roles, injuries::register_injuries,
        transition::transition_to_editing,
    },
};

/// Registers the editor's `Load` asset pass onto `app` — eight seam registrations,
/// the bespoke injuries pass (GTW-654), plus the transition gate (see the
/// [module docs](super) for the seam-vs-policy split).
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
    // The single-asset tile-role chain (GTW-564 seam), installed from the chain
    // owner's PUBLISHED config — path + map hook stay single-sourced in
    // gdtf_battle_presenter — with the editor's ADR-0003 fallback attached at
    // THIS registration (editor-owned policy, GTW-579 C4b). The game-theme
    // chain is NOT registered: the egui shell styles itself, so the editor
    // reads no theme field (GTW-625 — the GTW-579 AC2 amendment).
    app.init_hot_ron_chain(tile_roles_hot_ron_chain().with_fallback(default_tile_roles));

    // The seven folder families (GTW-570 seam) — the SAME glue-crate family
    // definitions the game registers, so game and editor build each registry
    // through literally one function. The terrain + theme defs share the ONE
    // MIXED `content/terrain/` tree; the seam's unconditional TypeId filter
    // keeps each walk to its own members.
    app.register_content_family::<WeaponsFamily>();
    app.register_content_family::<ArmorFamily>();
    app.register_content_family::<TerrainDefsFamily>();
    app.register_content_family::<ThemeDefsFamily>();
    // GTW-636 C2: the GANG mode's registry — the GTW-629 seam brings the loader,
    // salvage, validation window, and headless fallback with this one line.
    app.register_content_family::<GangsFamily>();
    // GTW-636 C1: the melee-weapons registry — the member model carries a
    // `melee_weapon` key (GTW-505), so the Gang mode's melee dropdown needs the
    // same family the game resolves that key against.
    app.register_content_family::<MeleeWeaponsFamily>();
    // GTW-663: the sprite-defs registry — the catalog a terrain def's
    // `graphic_name` foreign key resolves against, so the authoring-time
    // validation pass (and the GTW-664 sprite mode) reads the same family the
    // game loads.
    app.register_content_family::<SpriteDefsFamily>();

    // GTW-654: the BESPOKE injuries family (one folder → the InjuryRegistry +
    // InjuryTables pair — a declared GTW-570 seam exclusion, so it registers
    // through its own thin pass instead of `register_content_family`). The
    // INJURY mode's load combo / effects palette / weighting tables read these.
    register_injuries(app);

    app.add_systems(
        Update,
        transition_to_editing.run_if(in_state(EditorState::Load)),
    );
}
