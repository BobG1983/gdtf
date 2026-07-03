//! In-crate tests for the TERRAIN form's pure projection + the C2 footfall gate (GTW-474).
//!
//! The C4 in-engine / loader round-trip lives in the crate's integration test
//! (`tests/terrain_mode.rs`); these unit-test the pure draft → def projection, the C3 RON
//! round-trip on the loader's parser, and the C2 footfall-gate rule on the draft. Panic /
//! expect-free per the workspace lints (`assert!` + `let … else`).

use gdtf_battle_presenter::TileRole;
use gdtf_battle_sim::terrain::def::{
    TerrainDef, TerrainPresenterKind, TerrainSimKind, TerrainTag, TerrainUuid,
};

use super::{
    save::{draft_to_terrain_def, serialize_terrain_def},
    types::{FootfallChoice, TerrainDraft, TerrainKindChoice, offered_graphic_roles},
};

/// A terrain UUID from a small constant (the test's minted key).
fn key() -> TerrainUuid {
    TerrainUuid::new(bevy::asset::uuid::Uuid::from_u128(0x0184_0bcd_0001))
}

/// GTW-566 C5 (AC3) — the picker's offered list is DERIVED from the shared vocabulary:
/// it equals [`TileRole::ALL`] filtered by [`TileRole::def_authorable`], and it now
/// contains the GTW-543 emplacement plus the four GTW-470 oriented stairs (the roles the
/// old hand-mirrored 10-variant list could not author) while still excluding the
/// runtime-swap / link-direction roles and the unoffered plain door.
#[test]
fn picker_offers_exactly_the_def_authorable_vocabulary() {
    let offered = offered_graphic_roles();
    let derived: Vec<TileRole> = TileRole::ALL
        .into_iter()
        .filter(|role| role.def_authorable())
        .collect();
    assert_eq!(
        offered, derived,
        "the offered pick list must be TileRole::ALL filtered by def_authorable (C5)",
    );

    // The headline additions: emplacement + the four oriented stairs are NOW authorable.
    for newly_authorable in [
        TileRole::Emplacement,
        TileRole::StairNsUp,
        TileRole::StairNsDown,
        TileRole::StairEwUp,
        TileRole::StairEwDown,
    ] {
        assert!(
            offered.contains(&newly_authorable),
            "{newly_authorable:?} must be offered by the derived picker (GTW-566 C5)",
        );
    }
    // The presenter-picked / unoffered roles stay out of the picker.
    for excluded in [
        TileRole::EmplacementOccupied,
        TileRole::SlabDestroyed,
        TileRole::StairUp,
        TileRole::StairDown,
        TileRole::Door,
    ] {
        assert!(
            !offered.contains(&excluded),
            "{excluded:?} must NOT be offered by the picker (GTW-566 C5)",
        );
    }
}

/// GTW-516 C1/C2 — the graphic-role picker's selection wiring: clicking a role's sprite cell calls
/// `draft.set_graphic(choice)` (the picker's SOLE draft write), which the projection turns into the
/// def's `presenter_kind.graphic_name`. This drives that EXACT setter for EVERY role the grid
/// offers (the GTW-566 derived list) and asserts the projected def carries the matching role KEY —
/// so a click on any cell updates the draft's `graphic_name` to the right role (the sprite grid's
/// per-role identity).
///
/// A grid cell's index resolves through the SAME [`TileRole`] the sprite thumbnail draws with
/// (`index_in` over the loaded table), so pinning the setter → projected key mapping pins the
/// click → selection contract without a live egui context (the DRAW is Screenshot-QA-covered).
#[test]
fn graphic_picker_selection_updates_the_draft_graphic_name() {
    for choice in offered_graphic_roles() {
        let mut draft = TerrainDraft::default();
        // A Wall kind so the def projects a `Wall` presenter kind that carries the graphic role
        // key (the kind does not change the graphic_name — every kind carries it).
        draft.set_kind(TerrainKindChoice::Wall);
        // The picker's click handler is exactly this call.
        draft.set_graphic(choice);
        assert_eq!(
            draft.graphic(),
            choice,
            "set_graphic updates the draft's active graphic choice (the highlighted cell)",
        );

        let def = draft_to_terrain_def(&draft, key());
        let TerrainPresenterKind::Wall { graphic_name } = &def.presenter_kind else {
            unreachable!("a Wall draft projects to a Wall presenter kind");
        };
        assert_eq!(
            &***graphic_name,
            choice.as_key(),
            "selecting the {choice:?} cell sets the def's graphic_name to that role's key ({})",
            choice.as_key(),
        );
        // The projected key stays in-vocabulary: the presenter re-classifies it to the SAME
        // role (the editor + battlescape agree on the sprite).
        assert_eq!(
            TileRole::from_key(graphic_name),
            Some(choice),
            "the projected graphic_name must classify back to the picked role",
        );
    }
}

/// C2 — the footfall field is OFFERED only for the Slab kind: a Slab draft keeps a set footfall,
/// while a Wall / Cover draft forces it to `None` (the fail-closed gate in `set_kind`).
#[test]
fn footfall_offered_only_for_slab() {
    // Only Slab offers footfall.
    assert!(TerrainKindChoice::Slab.offers_footfall());
    assert!(!TerrainKindChoice::Wall.offers_footfall());
    assert!(!TerrainKindChoice::Cover.offers_footfall());

    // A Slab draft KEEPS a chosen footfall.
    let mut draft = TerrainDraft::default();
    draft.set_kind(TerrainKindChoice::Slab);
    draft.set_footfall(FootfallChoice::Metal);
    assert_eq!(
        draft.footfall(),
        FootfallChoice::Metal,
        "a Slab kind keeps the chosen footfall (C2 — offered for Slab)",
    );

    // Switching to a non-slab kind FORCES footfall to None (fail-closed C2).
    draft.set_kind(TerrainKindChoice::Wall);
    assert_eq!(
        draft.footfall(),
        FootfallChoice::None,
        "switching off Slab forces footfall to None (C2 — not offered for Wall/Cover)",
    );

    // And a stale footfall commit on a non-slab kind is IGNORED (the gate holds fail-closed).
    draft.set_footfall(FootfallChoice::Grate);
    assert_eq!(
        draft.footfall(),
        FootfallChoice::None,
        "a footfall commit on a non-slab kind is ignored (C2)",
    );
}

/// C2 — the kind picker offers ONLY Wall / Cover / Slab (no Floor / Scatter, retired on the UUID
/// model). A structural pin on the closed kind set.
#[test]
fn kind_picker_offers_only_wall_cover_slab() {
    assert_eq!(
        TerrainKindChoice::SEGMENT_ORDER,
        [
            TerrainKindChoice::Wall,
            TerrainKindChoice::Cover,
            TerrainKindChoice::Slab,
        ],
        "the kind picker offers EXACTLY Wall / Cover / Slab (Floor + Scatter retired — C2/C6)",
    );
}

/// C2/C3 — a Wall draft projects to the right `TerrainDef` shape: a `Wall` sim kind with the
/// draft's HP / armor / band, a `Wall` presenter kind with the graphic role, and the selected
/// tags.
#[test]
fn wall_draft_projects_to_wall_def() {
    let mut draft = TerrainDraft::default();
    draft.set_display_name("Bulkhead Wall".to_owned());
    draft.set_kind(TerrainKindChoice::Wall);
    draft.set_graphic(TileRole::Wall);
    draft.toggle_tag(TerrainTag::BlocksVision);

    let def = draft_to_terrain_def(&draft, key());
    assert_eq!(def.key, key());
    assert!(
        matches!(def.sim_kind, TerrainSimKind::Wall { .. }),
        "a Wall draft projects to a Wall sim kind",
    );
    assert!(
        matches!(
            &def.presenter_kind,
            TerrainPresenterKind::Wall { graphic_name } if &***graphic_name == "wall"
        ),
        "the Wall presenter kind carries the chosen graphic role key",
    );
    assert!(
        def.tags.contains(&TerrainTag::BlocksVision),
        "the selected tags are projected onto the def",
    );
}

/// C2/C3 — a Slab draft with a footfall projects to a `Slab` presenter kind carrying that
/// footfall (the only kind that does).
#[test]
fn slab_draft_projects_with_footfall() {
    let mut draft = TerrainDraft::default();
    draft.set_display_name("Deck Slab".to_owned());
    draft.set_kind(TerrainKindChoice::Slab);
    draft.set_graphic(TileRole::Slab);
    draft.set_footfall(FootfallChoice::Metal);

    let def = draft_to_terrain_def(&draft, key());
    assert!(
        matches!(def.sim_kind, TerrainSimKind::Slab { .. }),
        "a Slab draft projects to a Slab sim kind",
    );
    assert!(
        matches!(
            &def.presenter_kind,
            TerrainPresenterKind::Slab { footfall: Some(f), .. } if &***f == "footfall_metal"
        ),
        "the Slab presenter kind carries the chosen footfall (C2 — Slab-only)",
    );
}

/// C3 — the projected def RON round-trips through the SAME parser the GTW-487 terrain loader uses
/// (`ron::de::from_str::<TerrainDef>`): the reloaded def EQUALS the saved one (so the loader
/// resolves it byte-for-byte). Pin-discriminating — any dropped field flips the `assert_eq!`.
#[test]
fn terrain_def_round_trips_through_the_loader_parser() {
    let mut draft = TerrainDraft::default();
    draft.set_display_name("Bulkhead Wall".to_owned());
    draft.set_kind(TerrainKindChoice::Cover);
    draft.set_graphic(TileRole::Cover);
    draft.toggle_tag(TerrainTag::Indestructible);

    let def = draft_to_terrain_def(&draft, key());
    let serialized = serialize_terrain_def(&def);
    assert!(
        serialized.is_ok(),
        "serializing the terrain def must succeed: {:?}",
        serialized.as_ref().err(),
    );
    let Ok(serialized) = serialized else { return };

    let reloaded = ron::de::from_str::<TerrainDef>(&serialized);
    assert!(
        reloaded.is_ok(),
        "the serialized def must round-trip through the TerrainDef deserializer (the GTW-487 \
         loader's parser): {:?}",
        reloaded.as_ref().err(),
    );
    let Ok(reloaded) = reloaded else { return };
    assert_eq!(
        reloaded, def,
        "the reloaded TerrainDef must equal the saved one — every field survives (C3)",
    );
}
