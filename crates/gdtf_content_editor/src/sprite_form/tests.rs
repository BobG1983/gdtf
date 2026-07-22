//! GTW-664 C4: form-model headless tests over the SPRITE draft's pure mutators, the
//! path derivation, and the anchor-bounds clamp (the armor / injury form-test parity).

use std::path::Path;

use gdtf_content_families::sprites::{
    SpriteDef, SpriteFacing, SpriteImagePath, SpriteName, SpritePx, SpriteRect, SpriteSource,
};

use super::{
    draft::SpriteDraft,
    save::{draft_to_sprite_def, sprite_file_name, sprite_save_path_in},
};

/// A 16×16 sheet-cut source at sheet position `(32, 16)` — the shape every seeded def
/// authors.
fn sheet_source() -> SpriteSource {
    SpriteSource::Sheet {
        sheet: SpriteImagePath::new("sprites/alt_tileset_terrain.png".to_owned()),
        rect:  SpriteRect {
            x: SpritePx::new(32),
            y: SpritePx::new(16),
            w: SpritePx::new(16),
            h: SpritePx::new(16),
        },
    }
}

/// A Sheet-sourced draft's anchor clamps into the authored rect's `(w, h)` — the
/// CHEAPLY-KNOWABLE bounds (the rect IS the sprite's pixel extent) — while an in-bounds
/// write lands verbatim.
#[test]
fn anchor_clamps_to_the_sheet_rect_bounds() {
    let mut draft = SpriteDraft::new_sprite();
    draft.set_base_source(sheet_source());
    assert_eq!(
        draft.anchor_bounds(),
        Some((SpritePx::new(16), SpritePx::new(16))),
        "a sheet-cut sprite's anchor bounds are its rect extent",
    );

    draft.set_anchor(SpritePx::new(99), SpritePx::new(200));
    assert_eq!(*draft.def().anchor.x, 16, "x clamps to the rect width");
    assert_eq!(*draft.def().anchor.y, 16, "y clamps to the rect height");

    draft.set_anchor(SpritePx::new(8), SpritePx::new(12));
    assert_eq!(*draft.def().anchor.x, 8, "an in-bounds x lands verbatim");
    assert_eq!(*draft.def().anchor.y, 12, "an in-bounds y lands verbatim");
}

/// A File-sourced draft has NO model-side anchor clamp: the image's dims live in an
/// async-decoded asset the pure model cannot reach headlessly, so no bound is INVENTED
/// (GTW-664 C4 — the UI layer ranges the drag by the loaded dims once the preview
/// texture resolves).
#[test]
fn file_source_anchor_has_no_invented_clamp() {
    let mut draft = SpriteDraft::new_sprite();
    draft.set_base_source(SpriteSource::File(SpriteImagePath::new(
        "sprites/lone_crate.png".to_owned(),
    )));
    assert_eq!(
        draft.anchor_bounds(),
        None,
        "file dims are not knowable headlessly"
    );

    draft.set_anchor(SpritePx::new(999), SpritePx::new(998));
    assert_eq!(
        *draft.def().anchor.x,
        999,
        "no invented x clamp for a File source"
    );
    assert_eq!(
        *draft.def().anchor.y,
        998,
        "no invented y clamp for a File source"
    );
}

/// Replacing the base source RE-CLAMPS the anchor into the new bounds — shrinking a
/// sheet rect can never strand the authored anchor outside the sprite.
#[test]
fn shrinking_the_rect_reclamps_the_anchor() {
    let mut draft = SpriteDraft::new_sprite();
    draft.set_base_source(sheet_source());
    draft.set_anchor(SpritePx::new(16), SpritePx::new(16));

    draft.set_base_source(SpriteSource::Sheet {
        sheet: SpriteImagePath::new("sprites/alt_tileset_terrain.png".to_owned()),
        rect:  SpriteRect {
            x: SpritePx::new(0),
            y: SpritePx::new(0),
            w: SpritePx::new(8),
            h: SpritePx::new(4),
        },
    });
    assert_eq!(
        *draft.def().anchor.x,
        8,
        "x re-clamped to the shrunken width"
    );
    assert_eq!(
        *draft.def().anchor.y,
        4,
        "y re-clamped to the shrunken height"
    );
}

/// Facing overrides insert / replace / clear per facing, and an emptied map folds back
/// to `facings: None` (an absent map and an empty map author the same fallback).
#[test]
fn facing_overrides_fold_to_none_when_emptied() {
    let mut draft = SpriteDraft::new_sprite();
    let east = SpriteSource::File(SpriteImagePath::new("sprites/east.png".to_owned()));
    let north = SpriteSource::File(SpriteImagePath::new("sprites/north.png".to_owned()));

    draft.set_facing_override(SpriteFacing::East, Some(east.clone()));
    draft.set_facing_override(SpriteFacing::North, Some(north));
    assert_eq!(draft.facing_override(SpriteFacing::East), Some(&east));
    assert!(draft.facing_override(SpriteFacing::South).is_none());

    draft.set_facing_override(SpriteFacing::North, None);
    assert!(draft.facing_override(SpriteFacing::North).is_none());
    assert!(draft.def().facings.is_some(), "east override still present");

    draft.set_facing_override(SpriteFacing::East, None);
    assert_eq!(
        draft.def().facings,
        None,
        "clearing the last override folds the map back to None",
    );
}

/// The animation lifecycle: enable seeds ONE base-source frame; add / reorder / retarget
/// / remove edit the ordered list; remove is a no-op at one frame; disable folds back to
/// `None`.
#[test]
fn animation_rows_add_reorder_retarget_and_remove() {
    let mut draft = SpriteDraft::new_sprite();
    draft.set_base_source(sheet_source());
    draft.enable_animation();
    let frames = |draft: &SpriteDraft| -> Vec<SpriteSource> {
        draft
            .def()
            .animation
            .as_ref()
            .map(|animation| animation.frames.clone())
            .unwrap_or_default()
    };
    assert_eq!(
        frames(&draft),
        vec![sheet_source()],
        "enabling seeds the base source as the single starting frame",
    );

    // Enable is idempotent (the egui checkbox re-fires under multipass — bevy-traps #8).
    draft.add_frame();
    draft.enable_animation();
    assert_eq!(frames(&draft).len(), 2, "a re-enable never re-seeds");

    let second = SpriteSource::File(SpriteImagePath::new("sprites/cel_two.png".to_owned()));
    draft.set_frame(1, second.clone());
    draft.move_frame_up(1);
    assert_eq!(
        frames(&draft),
        vec![second.clone(), sheet_source()],
        "move-up swaps the row earlier",
    );
    draft.move_frame_down(0);
    assert_eq!(
        frames(&draft),
        vec![sheet_source(), second],
        "move-down swaps it back",
    );

    draft.remove_frame(1);
    assert_eq!(frames(&draft).len(), 1);
    draft.remove_frame(0);
    assert_eq!(
        frames(&draft).len(),
        1,
        "removing the last remaining frame is a no-op (turn the animation off instead)",
    );
    draft.disable_animation();
    assert_eq!(draft.def().animation, None);
}

/// The path derivation runs through the family consts + the shared sanitize helper: the
/// literal `content/sprites/<stem>.spritedef.ron` shape the GTW-663 loader dispatches on
/// (pinning the LITERALS is the drift alarm — GTW-621), with the `unnamed_sprite`
/// fallback for a name that sanitizes to nothing.
#[test]
fn save_path_derives_from_the_family_consts() {
    let name = SpriteName::new("Blast Door (open)".to_owned());
    assert_eq!(sprite_file_name(&name), "blast_door_open.spritedef.ron");
    assert_eq!(
        sprite_save_path_in(Path::new("/tmp/assets"), &name),
        Path::new("/tmp/assets/content/sprites/blast_door_open.spritedef.ron"),
    );
    assert_eq!(
        sprite_file_name(&SpriteName::new("!!!".to_owned())),
        "unnamed_sprite.spritedef.ron",
        "a name that sanitizes to nothing falls back to the documented stem",
    );
}

/// The save projection trims the name buffer into the registry-key [`SpriteName`] and
/// copies the def verbatim; a loaded def round-trips untouched (load is verbatim — even
/// an anchor outside the CURRENT bounds is the author's file truth).
#[test]
fn projection_trims_the_name_and_load_is_verbatim() {
    let mut draft = SpriteDraft::new_sprite();
    draft.set_name("  rusty_vent  ".to_owned());
    draft.set_base_source(sheet_source());
    let (name, def) = draft_to_sprite_def(&draft);
    assert_eq!(
        name.as_str(),
        "rusty_vent",
        "the projection trims the buffer"
    );
    assert_eq!(
        &def,
        draft.def(),
        "the projection is a copy of the working def"
    );

    // An authored file with an anchor outside its rect loads VERBATIM (the author's
    // truth) — only interactive writes clamp.
    let authored = SpriteDef {
        anchor: gdtf_content_families::sprites::SpriteAnchor {
            x: SpritePx::new(40),
            y: SpritePx::new(40),
        },
        ..def
    };
    let mut reloaded = SpriteDraft::default();
    assert!(reloaded.autoload_pending());
    reloaded.load_sprite(&name, &authored);
    assert!(
        !reloaded.autoload_pending(),
        "a load ends the one-shot seed"
    );
    assert_eq!(reloaded.def(), &authored, "load copies the def verbatim");
    assert_eq!(reloaded.name(), "rusty_vent");
}
