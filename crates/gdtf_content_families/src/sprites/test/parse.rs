use super::super::{
    SpriteAnchor, SpriteAnimation, SpriteDef, SpriteFacing, SpriteFacings, SpriteFps,
    SpriteImagePath, SpritePx, SpriteRect, SpriteSource,
};

/// A minimal sheet-rect def (the shape every seed authors).
const SHEET_DEF: &str = r#"(
    source: Sheet(
        sheet: "sprites/alt_tileset_terrain.png",
        rect: (x: 96, y: 0, w: 16, h: 16),
    ),
    anchor: (x: 8, y: 8),
)"#;

/// [`None`] (`#[serde(default)]` — the seeds author neither).
#[test]
fn sheet_rect_def_parses_with_optionals_defaulting_to_none() {
    let def = ron::de::from_str::<SpriteDef>(SHEET_DEF);
    assert!(def.is_ok(), "the sheet-rect def must parse: {def:?}");
    let Ok(def) = def else { return };
    assert_eq!(
        def.source,
        SpriteSource::Sheet {
            sheet: SpriteImagePath::new("sprites/alt_tileset_terrain.png".to_owned()),
            rect:  SpriteRect {
                x: SpritePx::new(96),
                y: SpritePx::new(0),
                w: SpritePx::new(16),
                h: SpritePx::new(16),
            },
        },
    );
    assert_eq!(
        def.anchor,
        SpriteAnchor {
            x: SpritePx::new(8),
            y: SpritePx::new(8),
        },
    );
    assert!(
        def.facings.is_none(),
        "omitted facings must default to None"
    );
    assert!(
        def.animation.is_none(),
        "omitted animation must default to None",
    );
}

#[test]
fn full_schema_with_file_source_facings_and_animation_parses() {
    let def = ron::de::from_str::<SpriteDef>(
        r#"(
            source: File("sprites/lone_crate.png"),
            anchor: (x: 8, y: 15),
            facings: Some({
                North: File("sprites/lone_crate_n.png"),
                East: Sheet(
                    sheet: "sprites/crates.png",
                    rect: (x: 16, y: 0, w: 16, h: 16),
                ),
            }),
            animation: Some((
                fps: 4.0,
                frames: [
                    File("sprites/lone_crate.png"),
                    File("sprites/lone_crate_2.png"),
                ],
            )),
        )"#,
    );
    assert!(def.is_ok(), "the full-schema def must parse: {def:?}");
    let Ok(def) = def else { return };
    assert_eq!(
        def.source,
        SpriteSource::File(SpriteImagePath::new("sprites/lone_crate.png".to_owned())),
    );
    let expected_facings = SpriteFacings::new([
        (
            SpriteFacing::North,
            SpriteSource::File(SpriteImagePath::new("sprites/lone_crate_n.png".to_owned())),
        ),
        (
            SpriteFacing::East,
            SpriteSource::Sheet {
                sheet: SpriteImagePath::new("sprites/crates.png".to_owned()),
                rect:  SpriteRect {
                    x: SpritePx::new(16),
                    y: SpritePx::new(0),
                    w: SpritePx::new(16),
                    h: SpritePx::new(16),
                },
            },
        ),
    ]);
    assert_eq!(def.facings, Some(expected_facings));
    let animation = def.animation;
    assert!(
        animation.is_some(),
        "the authored animation must parse to Some",
    );
    let Some(animation) = animation else { return };
    assert_eq!(
        (*animation.fps).to_bits(),
        4.0_f32.to_bits(),
        "the authored fps must round through",
    );
    assert_eq!(animation.frames.len(), 2, "both frames must parse");
}

#[test]
fn missing_anchor_fails_to_parse() {
    let def = ron::de::from_str::<SpriteDef>(r#"(source: File("sprites/x.png"))"#);
    assert!(
        def.is_err(),
        "a def without the required anchor must fail to parse, got {def:?}",
    );
}

#[test]
fn sprite_def_round_trips_through_serde() {
    let def = SpriteDef {
        source:    SpriteSource::Sheet {
            sheet: SpriteImagePath::new("sprites/alt_tileset_terrain.png".to_owned()),
            rect:  SpriteRect {
                x: SpritePx::new(0),
                y: SpritePx::new(16),
                w: SpritePx::new(16),
                h: SpritePx::new(16),
            },
        },
        anchor:    SpriteAnchor {
            x: SpritePx::new(8),
            y: SpritePx::new(8),
        },
        facings:   Some(SpriteFacings::new([(
            SpriteFacing::South,
            SpriteSource::File(SpriteImagePath::new("sprites/s.png".to_owned())),
        )])),
        animation: Some(SpriteAnimation {
            fps:    SpriteFps::new(2.0),
            frames: vec![SpriteSource::File(SpriteImagePath::new(
                "sprites/s.png".to_owned(),
            ))],
        }),
    };
    let serialized = ron::ser::to_string(&def);
    assert!(
        serialized.is_ok(),
        "serializing must succeed: {serialized:?}"
    );
    let Ok(serialized) = serialized else { return };
    let reparsed = ron::de::from_str::<SpriteDef>(&serialized);
    assert!(reparsed.is_ok(), "re-parsing must succeed: {reparsed:?}");
    let Ok(reparsed) = reparsed else { return };
    assert_eq!(reparsed, def, "the def must round-trip identically");
}
