use super::super::assert_schema_is_usable;
use crate::net_qa::wire::{
    EditorDraftNameNet, EditorFieldNet, GangAttributeNet, GangAttributeValueNet,
};

#[test]
fn the_field_traces_a_usable_shape() {
    assert_schema_is_usable::<EditorFieldNet>("EditorFieldNet");
    assert_schema_is_usable::<EditorDraftNameNet>("EditorDraftNameNet");
    assert_schema_is_usable::<GangAttributeNet>("GangAttributeNet");
    assert_schema_is_usable::<GangAttributeValueNet>("GangAttributeValueNet");
}
