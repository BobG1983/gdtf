use bevy::math::Vec2;
use serde::Deserialize;

use super::{assert_ron_round_trip, assert_schema_is_usable};
use crate::{
    mcp::wire::{
        EditorPanNet, EditorZoomNet,
        camera::{EditorPanXNet, EditorPanYNet},
    },
    preview::view::PreviewPan,
};

// A client's own reading of the pan offset, with the two axes distinct.
#[derive(Debug, Deserialize)]
struct PanRow {
    x: f32,
    y: f32,
}

// Every value here is exactly representable, so no case turns on a rounding step.
const OFFSET: Vec2 = Vec2::new(0.5, -2.0);

fn read_back(offset: Vec2) -> PanRow {
    let mirrored = EditorPanNet::from_pan(PreviewPan::with_offset(offset));
    let Ok(text) = ron::ser::to_string(&mirrored) else {
        unreachable!("a pan offset serializes to compact RON");
    };
    match ron::de::from_str::<PanRow>(&text) {
        Ok(row) => row,
        Err(fault) => unreachable!("`{text}` reads back as two named axes: {fault}"),
    }
}

#[test]
fn every_exactly_representable_zoom_round_trips() {
    for scale in [0.5_f32, 1.0, 2.0] {
        assert_ron_round_trip(&EditorZoomNet::new(scale));
    }
}

#[test]
fn each_pan_axis_round_trips() {
    assert_ron_round_trip(&EditorPanXNet::new(0.5));
    assert_ron_round_trip(&EditorPanYNet::new(-2.0));
}

#[test]
fn a_pan_offset_round_trips() {
    assert_ron_round_trip(&EditorPanNet::from_pan(PreviewPan::with_offset(OFFSET)));
}

#[test]
fn a_pan_offset_carries_each_axis_on_its_own_name() {
    let row = read_back(OFFSET);
    assert!(
        (row.x - OFFSET.x).abs() < f32::EPSILON,
        "x must arrive as x. The two axes differ here, so a swapped pair reads as a valid pan \
         at the wrong place: {row:?}",
    );
    assert!(
        (row.y - OFFSET.y).abs() < f32::EPSILON,
        "y must arrive as y: {row:?}",
    );
}

#[test]
fn the_camera_types_trace_usable_shapes() {
    assert_schema_is_usable::<EditorZoomNet>("EditorZoomNet");
    assert_schema_is_usable::<EditorPanXNet>("EditorPanXNet");
    assert_schema_is_usable::<EditorPanYNet>("EditorPanYNet");
    assert_schema_is_usable::<EditorPanNet>("EditorPanNet");
}
