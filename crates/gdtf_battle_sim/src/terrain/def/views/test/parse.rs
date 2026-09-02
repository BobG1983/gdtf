use super::super::{TerrainView, TerrainViews};
use crate::terrain::{def::TerrainDef, facing::TerrainFacing, piece::TerrainGraphicKey};

// The def every literal here is built from. `{views}` stands where the authored entry goes.
const DEF_TEMPLATE: &str = r#"(
    key: "01840a3e-0000-4000-8000-0000000000a1",
    display_name: "View Probe",
    sim_kind: Cover(hp: 20, armor_protection: 3, armor_hardness: 1, height_band: Low),
    presenter_kind: Cover(graphic_name: "cover"),
{views})"#;

// The one entry the short reading below deletes and nothing else.
const VIEWS_ENTRY: &str = r#"    views: [
        (view: Facing(North), sprite: "cover_n"),
        (view: Facing(East), sprite: "cover_e"),
        (view: Facing(South), sprite: "cover_s"),
        (view: Facing(West), sprite: "cover_w"),
    ],
"#;

fn with_views() -> String {
    DEF_TEMPLATE.replace("{views}", VIEWS_ENTRY)
}

fn without_views() -> String {
    DEF_TEMPLATE.replace("{views}", "")
}

// The rows `VIEWS_ENTRY` authors, in the order it writes them.
fn authored_rows() -> [(TerrainView, &'static str); 4] {
    [
        (TerrainView::Facing(TerrainFacing::North), "cover_n"),
        (TerrainView::Facing(TerrainFacing::East), "cover_e"),
        (TerrainView::Facing(TerrainFacing::South), "cover_s"),
        (TerrainView::Facing(TerrainFacing::West), "cover_w"),
    ]
}

#[test]
fn a_def_authoring_a_full_view_list_reads_every_row_back() {
    let text = with_views();
    let parsed = ron::de::from_str::<TerrainDef>(&text);
    assert!(
        parsed.is_ok(),
        "a def authoring a views list must parse: {parsed:?}",
    );
    let Ok(def) = parsed else { return };
    for (view, sprite) in authored_rows() {
        assert_eq!(
            def.views.sprite(view),
            Some(&TerrainGraphicKey::new(sprite.to_owned())),
            "the authored {view:?} row must read back naming `{sprite}`, got {:?}",
            def.views,
        );
    }
    assert_eq!(
        def.views.len(),
        authored_rows().len(),
        "the parsed list holds exactly the authored rows, got {:?}",
        def.views,
    );
    let empty = TerrainViews::new(Vec::new());
    assert_eq!(
        empty.sprite(TerrainView::Single),
        None,
        "a list with no row for a view names no sprite for it",
    );
}

#[test]
fn the_same_literal_parses_with_its_views_entry_and_fails_without_it() {
    let short = ron::de::from_str::<TerrainDef>(&without_views());
    assert!(
        short.is_err(),
        "`views` carries no #[serde(default)], so a def omitting it must fail to parse — which \
         is what forces every shipped file to migrate: {short:?}",
    );
    let full = ron::de::from_str::<TerrainDef>(&with_views());
    assert!(
        full.is_ok(),
        "the same literal with its views entry back must parse, or the reading above went red \
         for some other reason: {full:?}",
    );
}
