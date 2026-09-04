use gdtf_content_families::sprites::{
    SpriteFps, SpriteImagePath, SpritePx, SpriteRect, SpriteSource,
};

use super::super::assert_ron_round_trip;
use crate::mcp::wire::{
    EditorFieldNet, EditorListIndexNet, SpriteAnimatedNet, SpriteFacingNet, SpriteFieldNet,
    SpriteFpsNet, SpritePxNet, SpriteSourceNet,
};

fn a_source() -> SpriteSourceNet {
    SpriteSourceNet::from_source(&SpriteSource::Sheet {
        sheet: SpriteImagePath::new("sprites/sheet.png".to_owned()),
        rect:  SpriteRect {
            x: SpritePx::new(0),
            y: SpritePx::new(0),
            w: SpritePx::new(16),
            h: SpritePx::new(16),
        },
    })
}

#[test]
fn every_sprite_field_arm_round_trips() {
    assert_ron_round_trip(&EditorFieldNet::Sprite(SpriteFieldNet::BaseSource(
        a_source(),
    )));
    assert_ron_round_trip(&EditorFieldNet::Sprite(SpriteFieldNet::AnchorX(
        SpritePxNet::from_px(SpritePx::new(2)),
    )));
    assert_ron_round_trip(&EditorFieldNet::Sprite(SpriteFieldNet::AnchorY(
        SpritePxNet::from_px(SpritePx::new(3)),
    )));
    assert_ron_round_trip(&EditorFieldNet::Sprite(SpriteFieldNet::Fps(
        SpriteFpsNet::from_fps(SpriteFps::new(12.0)),
    )));
    for facing in SpriteFacingNet::ALL {
        assert_ron_round_trip(&EditorFieldNet::Sprite(SpriteFieldNet::FacingOverride {
            facing,
            source: Some(a_source()),
        }));
        assert_ron_round_trip(&EditorFieldNet::Sprite(SpriteFieldNet::FacingOverride {
            facing,
            source: None,
        }));
    }
    assert_ron_round_trip(&EditorFieldNet::Sprite(SpriteFieldNet::Frame {
        index:  EditorListIndexNet::new(1),
        source: a_source(),
    }));
    assert_ron_round_trip(&EditorFieldNet::Sprite(SpriteFieldNet::Animated(
        SpriteAnimatedNet::new(true),
    )));
}
