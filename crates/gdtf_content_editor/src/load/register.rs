//! The `Load`-pass registrar — [`register_load`], the eight generic family registrations plus the
//! transition gate (extracted from the module wiring so `mod.rs` stays fn-free).

use bevy::prelude::*;
use gdtf_assets::ContentFamilyAppExt;
use gdtf_content_families::{
    ArmorFamily, AttachmentsFamily, GangsFamily, MeleeWeaponsFamily, SpriteDefsFamily,
    TerrainDefsFamily, ThemeDefsFamily, WeaponsFamily,
};

use crate::{
    EditorState,
    load::{injuries::register_injuries, transition::transition_to_editing},
};

/// Registers the editor's `Load` asset pass onto `app` — eight family registrations,
/// the bespoke injuries pass (GTW-654), plus the transition gate (see the
/// [module docs](super) for the registration-vs-policy split).
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
    // NO single-asset hot-RON chain is registered (GTW-665 retired the
    // presenter's tile-role chain — terrain graphics now resolve through the
    // SpriteDefsFamily registry below; the game's GdtfTheme chain was never
    // registered here: the egui shell styles itself, so the editor reads no
    // theme field — GTW-625, the GTW-579 AC2 amendment).

    // The eight folder families (the GTW-570 registration helper) — the SAME
    // glue-crate family definitions the game registers, so game and editor build
    // each registry through literally one function. The terrain + theme defs share
    // the ONE MIXED `content/terrain/` tree; the helper's unconditional TypeId
    // filter keeps each walk to its own members.
    app.register_content_family::<WeaponsFamily>();
    app.register_content_family::<ArmorFamily>();
    app.register_content_family::<TerrainDefsFamily>();
    app.register_content_family::<ThemeDefsFamily>();
    // GTW-636 C2: the GANG mode's registry — the GTW-629 registration helper brings
    // the loader, salvage, validation window, and headless fallback with this one line.
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
    // GTW-669 C1: the attachments registry — the ATTACHMENT mode's load-any /
    // save family, and the option source the GTW-670 weapon forms' attachment
    // combos resolve against (the same GTW-619 family the game registers).
    app.register_content_family::<AttachmentsFamily>();

    // GTW-654: the BESPOKE injuries family (one folder → the InjuryRegistry +
    // InjuryTables pair — a declared exclusion from the GTW-570 registration
    // helper, so it registers through its own thin pass instead of
    // `register_content_family`). The
    // INJURY mode's load combo / effects palette / weighting tables read these.
    register_injuries(app);

    app.add_systems(
        Update,
        transition_to_editing.run_if(in_state(EditorState::Load)),
    );
}
