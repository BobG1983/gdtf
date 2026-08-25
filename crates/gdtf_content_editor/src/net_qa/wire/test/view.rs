use bevy::math::Vec2;
use gdtf_battle_presenter::{ContextDepth, IsolateView, ViewMode};

use super::{assert_ron_round_trip, assert_schema_is_usable};
use crate::{
    net_qa::wire::{
        EditorIsolateViewNet, EditorPanNet, EditorViewModeNet, EditorViewNet, EditorZoomNet,
        view::EditorContextDepthNet,
    },
    preview::view::PreviewPan,
};

fn a_view() -> EditorViewNet {
    EditorViewNet::new(
        EditorViewModeNet::from_mode(ViewMode::FullView),
        EditorIsolateViewNet::from_isolate(IsolateView::On(ContextDepth::new(2))),
        EditorZoomNet::new(0.5),
        EditorPanNet::from_pan(PreviewPan::with_offset(Vec2::new(0.5, -2.0))),
    )
}

#[test]
fn both_view_modes_round_trip_under_their_own_names() {
    for mode in [ViewMode::DownToActive, ViewMode::FullView] {
        let mirrored = EditorViewModeNet::from_mode(mode);
        assert_eq!(
            format!("{mirrored:?}"),
            format!("{mode:?}"),
            "the wire mirror of {mode:?} must carry that mode's own name",
        );
        assert_ron_round_trip(&mirrored);
    }
}

#[test]
fn a_context_depth_round_trips() {
    assert_ron_round_trip(&EditorContextDepthNet::new(2));
}

#[test]
fn both_isolation_arms_round_trip() {
    assert_ron_round_trip(&EditorIsolateViewNet::from_isolate(IsolateView::Off));
    assert_ron_round_trip(&EditorIsolateViewNet::from_isolate(IsolateView::On(
        ContextDepth::new(2),
    )));
}

#[test]
fn an_isolated_view_carries_the_depth_it_mirrored() {
    assert_eq!(
        EditorIsolateViewNet::from_isolate(IsolateView::On(ContextDepth::new(2))),
        EditorIsolateViewNet::On(EditorContextDepthNet::new(2)),
        "the onion depth must come across, or a client cannot tell how far below the active \
         storey the canvas still draws",
    );
}

#[test]
fn a_whole_view_round_trips() {
    let view: EditorViewNet = a_view();
    assert_ron_round_trip(&view);
}

#[test]
fn the_view_types_trace_usable_shapes() {
    assert_schema_is_usable::<EditorViewModeNet>("EditorViewModeNet");
    assert_schema_is_usable::<EditorContextDepthNet>("EditorContextDepthNet");
    assert_schema_is_usable::<EditorIsolateViewNet>("EditorIsolateViewNet");
    assert_schema_is_usable::<EditorViewNet>("EditorViewNet");
}
