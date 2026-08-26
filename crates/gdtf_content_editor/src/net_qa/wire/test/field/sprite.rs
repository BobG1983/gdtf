use gdtf_content_families::sprites::{
    SpriteFps, SpriteImagePath, SpritePx, SpriteRect, SpriteSource,
};

use super::super::assert_ron_round_trip;
use crate::net_qa::wire::{
    EditorFieldNet, EditorListIndexNet, SpriteAnimatedNet, SpriteFacingNet, SpriteFpsNet,
    SpritePxNet, SpriteSourceNet,
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
    assert_ron_round_trip(&EditorFieldNet::SpriteBaseSource(a_source()));
    assert_ron_round_trip(&EditorFieldNet::SpriteAnchorX(SpritePxNet::from_px(
        SpritePx::new(2),
    )));
    assert_ron_round_trip(&EditorFieldNet::SpriteAnchorY(SpritePxNet::from_px(
        SpritePx::new(3),
    )));
    assert_ron_round_trip(&EditorFieldNet::SpriteFps(SpriteFpsNet::from_fps(
        SpriteFps::new(12.0),
    )));
    for facing in SpriteFacingNet::ALL {
        assert_ron_round_trip(&EditorFieldNet::SpriteFacingOverride {
            facing,
            source: Some(a_source()),
        });
        assert_ron_round_trip(&EditorFieldNet::SpriteFacingOverride {
            facing,
            source: None,
        });
    }
    assert_ron_round_trip(&EditorFieldNet::SpriteFrame {
        index:  EditorListIndexNet::new(1),
        source: a_source(),
    });
    assert_ron_round_trip(&EditorFieldNet::SpriteAnimated(SpriteAnimatedNet::new(
        true,
    )));
}
