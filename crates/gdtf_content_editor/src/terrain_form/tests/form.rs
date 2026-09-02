use gdtf_battle_presenter::TileRole;
use gdtf_battle_sim::terrain::def::{TerrainDef, TerrainPresenterKind, TerrainSimKind, TerrainTag};

use super::support::key;
use crate::terrain_form::{
    FootfallChoice, TerrainDraft, TerrainKindChoice, draft_to_terrain_def, offered_graphic_roles,
    serialize_terrain_def,
};

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

    for newly_authorable in [
        TileRole::Emplacement,
        TileRole::StairNsUp,
        TileRole::StairNsDown,
        TileRole::StairEwUp,
        TileRole::StairEwDown,
    ] {
        assert!(
            offered.contains(&newly_authorable),
            "{newly_authorable:?} must be offered by the derived picker ",
        );
    }
    for excluded in [TileRole::StairUp, TileRole::StairDown, TileRole::Door] {
        assert!(
            !offered.contains(&excluded),
            "{excluded:?} must NOT be offered by the picker ",
        );
    }
}

#[test]
fn graphic_picker_selection_updates_the_draft_graphic_name() {
    for choice in offered_graphic_roles() {
        let mut draft = TerrainDraft::default();
        draft.set_kind(TerrainKindChoice::Wall);
        draft.set_graphic(choice);
        assert_eq!(
            draft.graphic(),
            choice,
            "set_graphic updates the draft's active graphic choice (the highlighted cell)",
        );

        let Ok(def) = draft_to_terrain_def(&draft, key()) else {
            unreachable!("a Wall draft always projects (no fail-closed gate applies)")
        };
        let TerrainPresenterKind::Wall { graphic_name } = &def.presenter_kind else {
            unreachable!("a Wall draft projects to a Wall presenter kind");
        };
        assert_eq!(
            &***graphic_name,
            choice.as_key(),
            "selecting the {choice:?} cell sets the def's graphic_name to that role's key ({})",
            choice.as_key(),
        );
        assert_eq!(
            TileRole::from_key(graphic_name),
            Some(choice),
            "the projected graphic_name must classify back to the picked role",
        );
    }
}

#[test]
fn footfall_offered_only_for_slab() {
    assert!(TerrainKindChoice::Slab.offers_footfall());
    assert!(!TerrainKindChoice::Wall.offers_footfall());
    assert!(!TerrainKindChoice::Cover.offers_footfall());
    assert!(!TerrainKindChoice::Emplacement.offers_footfall());

    let mut draft = TerrainDraft::default();
    draft.set_kind(TerrainKindChoice::Slab);
    draft.set_footfall(FootfallChoice::Metal);
    assert_eq!(
        draft.footfall(),
        FootfallChoice::Metal,
        "a Slab kind keeps the chosen footfall (C2 — offered for Slab)",
    );

    draft.set_kind(TerrainKindChoice::Wall);
    assert_eq!(
        draft.footfall(),
        FootfallChoice::None,
        "switching off Slab forces footfall to None (C2 — not offered for Wall/Cover)",
    );

    draft.set_footfall(FootfallChoice::Grate);
    assert_eq!(
        draft.footfall(),
        FootfallChoice::None,
        "a footfall commit on a non-slab kind is ignored (C2)",
    );
}

#[test]
fn wall_draft_projects_to_wall_def() {
    let mut draft = TerrainDraft::default();
    draft.set_display_name("Bulkhead Wall".to_owned());
    draft.set_kind(TerrainKindChoice::Wall);
    draft.set_graphic(TileRole::Wall);
    draft.toggle_tag(TerrainTag::BlocksVision);

    let Ok(def) = draft_to_terrain_def(&draft, key()) else {
        unreachable!("a Wall draft always projects (no fail-closed gate applies)")
    };
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

#[test]
fn slab_draft_projects_with_footfall() {
    let mut draft = TerrainDraft::default();
    draft.set_display_name("Deck Slab".to_owned());
    draft.set_kind(TerrainKindChoice::Slab);
    draft.set_graphic(TileRole::Slab);
    draft.set_footfall(FootfallChoice::Metal);

    let Ok(def) = draft_to_terrain_def(&draft, key()) else {
        unreachable!("a Slab draft always projects (no fail-closed gate applies)")
    };
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

#[test]
fn terrain_def_round_trips_through_the_loader_parser() {
    let mut draft = TerrainDraft::default();
    draft.set_display_name("Bulkhead Wall".to_owned());
    draft.set_kind(TerrainKindChoice::Cover);
    draft.set_graphic(TileRole::Cover);
    draft.toggle_tag(TerrainTag::Indestructible);

    let Ok(def) = draft_to_terrain_def(&draft, key()) else {
        unreachable!("a Cover draft always projects (no fail-closed gate applies)")
    };
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
        "the serialized def must round-trip through the TerrainDef deserializer (the \
         loader's parser): {:?}",
        reloaded.as_ref().err(),
    );
    let Ok(reloaded) = reloaded else { return };
    assert_eq!(
        reloaded, def,
        "the reloaded TerrainDef must equal the saved one — every field survives (C3)",
    );
}

#[test]
fn blocking_overrides_project_and_round_trip() {
    use gdtf_battle_sim::terrain::def::{BlocksPathingOverride, LosBlocking};

    let default_draft = TerrainDraft::default();
    let Ok(default_def) = draft_to_terrain_def(&default_draft, key()) else {
        unreachable!("a default Wall draft always projects")
    };
    assert_eq!(
        default_def.blocks_pathing, None,
        "default = kind default (path)"
    );
    assert_eq!(default_def.blocks_los, None, "default = kind default (LoS)");

    let mut draft = TerrainDraft::default();
    draft.set_display_name("Glass Wall".to_owned());
    draft.set_kind(TerrainKindChoice::Wall);
    draft.set_graphic(TileRole::Wall);
    draft.set_blocks_pathing(Some(true));
    draft.set_blocks_los(Some(LosBlocking::None));

    let Ok(def) = draft_to_terrain_def(&draft, key()) else {
        unreachable!("a Wall draft always projects")
    };
    assert_eq!(
        def.blocks_pathing,
        Some(BlocksPathingOverride::new(true)),
        "AC3: the path-blocking override projects onto the def",
    );
    assert_eq!(
        def.blocks_los,
        Some(LosBlocking::None),
        "AC3: the LoS-blocking override projects onto the def",
    );

    let Ok(serialized) = serialize_terrain_def(&def) else {
        unreachable!("serializing the override def must succeed")
    };
    let Ok(reloaded) = ron::de::from_str::<TerrainDef>(&serialized) else {
        unreachable!("the override def must round-trip through the loader parser")
    };
    assert_eq!(
        reloaded, def,
        "the reloaded TerrainDef must carry both authored overrides (AC3)",
    );
}

#[test]
fn hp_and_armor_ranges_hold_the_seeded_draft() {
    let draft = TerrainDraft::default();

    assert!(
        TerrainDraft::HP_RANGE.contains(&*draft.cover_hp()),
        "the seeded cover HP sits inside the range the form offers",
    );
    assert!(
        TerrainDraft::HP_RANGE.contains(&*draft.slab_hp()),
        "the seeded slab HP sits inside the range the form offers",
    );
    assert!(
        TerrainDraft::ARMOR_RANGE.contains(&*draft.armor_protection()),
        "the seeded armor protection sits inside the range the form offers",
    );
    assert!(
        TerrainDraft::ARMOR_RANGE.contains(&*draft.armor_hardness()),
        "the seeded armor hardness sits inside the range the form offers",
    );
}
