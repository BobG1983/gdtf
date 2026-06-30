//! In-crate tests for the TERRAIN form's pure projection + the C2 footfall gate (GTW-474).
//!
//! The C4 in-engine / loader round-trip lives in the crate's integration test
//! (`tests/terrain_mode.rs`); these unit-test the pure draft → def projection, the C3 RON
//! round-trip on the loader's parser, and the C2 footfall-gate rule on the draft. Panic /
//! expect-free per the workspace lints (`assert!` + `let … else`).

use gdtf_battle_sim::terrain::def::{
    TerrainDef, TerrainPresenterKind, TerrainSimKind, TerrainTag, TerrainUuid,
};

use super::{
    save::{draft_to_terrain_def, serialize_terrain_def},
    types::{FootfallChoice, TerrainDraft, TerrainGraphicChoice, TerrainKindChoice},
};

/// A terrain UUID from a small constant (the test's minted key).
fn key() -> TerrainUuid {
    TerrainUuid::new(bevy::asset::uuid::Uuid::from_u128(0x0184_0bcd_0001))
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
    draft.set_graphic(TerrainGraphicChoice::Wall);
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
    draft.set_graphic(TerrainGraphicChoice::Slab);
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
    draft.set_graphic(TerrainGraphicChoice::Cover);
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
