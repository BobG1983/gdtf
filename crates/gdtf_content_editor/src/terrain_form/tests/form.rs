use gdtf_battle_sim::terrain::{
    def::{
        TerrainDef, TerrainPresenterKind, TerrainSimKind, TerrainTag, TerrainView, owed_views_for,
    },
    facing::TerrainFacing,
    piece::TerrainGraphicKey,
};

use super::support::key;
use crate::terrain_form::{
    FootfallChoice, TerrainDraft, TerrainKindChoice, draft_to_terrain_def, offers_view_expander,
    serialize_terrain_def, view_rows,
};

// A draft of one kind, carrying no tag, which is what the two picker cases start from.
fn draft_of(kind: TerrainKindChoice) -> TerrainDraft {
    let mut draft = TerrainDraft::default();
    draft.set_kind(kind);
    draft
}

// The sprite key a case names for one view, distinct per view.
fn view_key(view: TerrainView) -> TerrainGraphicKey {
    TerrainGraphicKey::new(format!("{view:?}").to_lowercase())
}

#[test]
fn view_rows_offers_the_owed_set_and_every_pick_reaches_the_def() {
    let mut draft = draft_of(TerrainKindChoice::Wall);
    let rows = view_rows(&draft);
    let owed = owed_views_for(TerrainKindChoice::Wall.piece_kind(), draft.tags());
    let missing: Vec<TerrainView> = owed
        .iter()
        .filter(|view| !rows.contains(view))
        .copied()
        .collect();
    assert!(
        missing.is_empty(),
        "the picker draws one row per view the draft owes, and these were not offered: \
         {missing:?}",
    );
    assert_eq!(
        *rows, *owed,
        "the picker's rows are exactly the owed set, in the order the sim derives them",
    );

    let slab = draft_of(TerrainKindChoice::Slab);
    assert!(
        !slab.has_tag(TerrainTag::Openable) && !slab.has_tag(TerrainTag::Stair),
        "the slab half needs a draft carrying neither tag, or it would owe a door's or a \
         stair's rows instead",
    );
    assert_eq!(
        *view_rows(&slab),
        vec![TerrainView::Single],
        "a plain slab owes one drawing, so its picker draws one row",
    );

    for view in rows.iter() {
        draft.set_view(*view, view_key(*view));
    }
    let Ok(def) = draft_to_terrain_def(&draft, key()) else {
        unreachable!("a Wall draft always projects (no fail-closed gate applies)")
    };
    let dropped: Vec<TerrainView> = rows
        .iter()
        .filter(|view| def.views.sprite(**view) != Some(&view_key(**view)))
        .copied()
        .collect();
    assert!(
        dropped.is_empty(),
        "every key the picker wrote through a row must reach the projected def; these did \
         not: {dropped:?}",
    );
}

#[test]
fn the_view_rows_sit_under_an_expander_only_for_a_def_owing_more_than_one() {
    assert!(
        offers_view_expander(&draft_of(TerrainKindChoice::Wall)),
        "a wall owes a run on each side and a turn at each corner, so its rows get a header",
    );
    let slab = draft_of(TerrainKindChoice::Slab);
    assert!(
        !offers_view_expander(&slab),
        "a plain slab owes one view, so its single row is drawn with no header",
    );
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
    draft.set_view(
        TerrainView::Edge(TerrainFacing::North),
        TerrainGraphicKey::new("wall".to_owned()),
    );
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
        matches!(&def.presenter_kind, TerrainPresenterKind::Wall),
        "a Wall draft projects to the Wall presenter kind",
    );
    assert_eq!(
        def.views
            .sprite(TerrainView::Edge(TerrainFacing::North))
            .map(|sprite| (**sprite).clone()),
        Some("wall".to_owned()),
        "the projected def carries the sprite key the draft set on its `Edge(North)` view",
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
    draft.set_view(
        TerrainView::Single,
        TerrainGraphicKey::new("slab".to_owned()),
    );
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
    draft.set_view(
        TerrainView::Facing(TerrainFacing::North),
        TerrainGraphicKey::new("cover".to_owned()),
    );
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
    draft.set_view(
        TerrainView::Edge(TerrainFacing::North),
        TerrainGraphicKey::new("wall".to_owned()),
    );
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
