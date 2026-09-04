use gdtf_content_families::sprites::{
    SpriteFacing, SpriteFps, SpriteImagePath, SpritePx, SpriteRect, SpriteSource,
};

use super::{assert_ron_round_trip, assert_schema_is_usable};
use crate::mcp::wire::{
    SpriteAnimatedNet, SpriteFacingNet, SpriteFpsNet, SpritePxNet, SpriteSourceNet,
    sprite::{SpriteImagePathNet, SpriteRectNet},
};

fn a_sheet_source() -> SpriteSource {
    SpriteSource::Sheet {
        sheet: SpriteImagePath::new("sprites/sheet.png".to_owned()),
        rect:  SpriteRect {
            x: SpritePx::new(4),
            y: SpritePx::new(8),
            w: SpritePx::new(16),
            h: SpritePx::new(32),
        },
    }
}

#[test]
fn every_facing_arm_round_trips_and_reads_back_as_the_facing_it_mirrored() {
    for facing in [
        SpriteFacing::North,
        SpriteFacing::East,
        SpriteFacing::South,
        SpriteFacing::West,
    ] {
        let mirrored = SpriteFacingNet::from_facing(facing);
        assert_ron_round_trip(&mirrored);
        assert_eq!(
            mirrored.to_facing(),
            facing,
            "a client's facing must come back as the family's own, or an override would land \
             on a different facing than the one it named",
        );
    }
}

#[test]
fn the_wires_own_facing_list_names_each_facing_once() {
    let mut seen: Vec<SpriteFacing> = Vec::with_capacity(SpriteFacingNet::ALL.len());
    for facing in SpriteFacingNet::ALL {
        let mirrored = facing.to_facing();
        assert!(
            !seen.contains(&mirrored),
            "{facing:?} maps onto a facing another arm already claims, so the list cannot stand \
             in for the family's own",
        );
        seen.push(mirrored);
    }
}

#[test]
fn every_source_arm_round_trips_and_reads_back_as_the_source_it_mirrored() {
    for source in [
        SpriteSource::File(SpriteImagePath::new("sprites/one.png".to_owned())),
        a_sheet_source(),
    ] {
        let mirrored = SpriteSourceNet::from_source(&source);
        assert_ron_round_trip(&mirrored);
        assert_eq!(
            mirrored.to_source(),
            source,
            "a client's source must come back as the family's own, rect and all",
        );
    }
}

#[test]
fn the_scalar_mirrors_round_trip_and_read_back_unchanged() {
    let px = SpritePxNet::from_px(SpritePx::new(12));
    assert_ron_round_trip(&px);
    assert_eq!(*px.to_px(), 12, "a pixel count survives the wire unchanged");

    let fps = SpriteFpsNet::from_fps(SpriteFps::new(24.0));
    assert_ron_round_trip(&fps);
    assert!(
        (*fps.to_fps() - 24.0).abs() < f32::EPSILON,
        "a frame rate survives the wire unchanged, got {}",
        *fps.to_fps(),
    );

    assert_ron_round_trip(&SpriteAnimatedNet::new(true));
    assert_ron_round_trip(&SpriteAnimatedNet::new(false));
}

#[test]
fn the_path_and_the_rect_round_trip_and_read_back_unchanged() {
    let path = SpriteImagePathNet::from_path(&SpriteImagePath::new("sprites/one.png".to_owned()));
    assert_ron_round_trip(&path);
    assert_eq!(
        *path.to_path(),
        "sprites/one.png",
        "a path survives the wire unchanged"
    );

    let rect = SpriteRect {
        x: SpritePx::new(1),
        y: SpritePx::new(2),
        w: SpritePx::new(3),
        h: SpritePx::new(4),
    };
    let mirrored = SpriteRectNet::from_rect(rect);
    assert_ron_round_trip(&mirrored);
    assert_eq!(
        mirrored.to_rect(),
        rect,
        "a sheet rect survives the wire with all four edges",
    );
}

#[test]
fn the_sprite_mirrors_trace_usable_shapes() {
    assert_schema_is_usable::<SpriteFacingNet>("SpriteFacingNet");
    assert_schema_is_usable::<SpriteSourceNet>("SpriteSourceNet");
    assert_schema_is_usable::<SpritePxNet>("SpritePxNet");
    assert_schema_is_usable::<SpriteFpsNet>("SpriteFpsNet");
    assert_schema_is_usable::<SpriteAnimatedNet>("SpriteAnimatedNet");
    assert_schema_is_usable::<SpriteImagePathNet>("SpriteImagePathNet");
    assert_schema_is_usable::<SpriteRectNet>("SpriteRectNet");
}
