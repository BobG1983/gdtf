//! Load fields into [`FieldsFamily`] by authored member key.
//! Value-agnostic: catalog presence only.
mod load_suite;

use gdtf_battle_sim::effects::fields::FieldDefRegistry;
use gdtf_content_families::FieldsFamily;
use load_suite::suite::{self, FamilyLoadContract};

impl FamilyLoadContract for FieldsFamily {
    fn is_empty(registry: &FieldDefRegistry) -> bool {
        registry.is_empty()
    }
}

#[test]
fn field_loader_no_ops_cleanly_without_asset_server() {
    suite::loader_no_ops_without_asset_server::<FieldsFamily>();
}

#[test]
fn load_does_not_leave_without_a_field_def_registry() {
    suite::load_gates_on_registry::<FieldsFamily>();
}

#[test]
fn real_asset_resolves_field_def_registry_keyed_by_filename() {
    suite::real_asset_resolves_registry::<FieldsFamily>();
}
