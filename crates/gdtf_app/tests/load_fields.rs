//! only binds it to [`FieldsFamily`] with the authored member key. The
//! assertions stay VALUE-AGNOSTIC (catalog presence + the authored
mod load_suite;

use gdtf_battle_sim::effects::fields::{FieldDefRegistry, FieldKey};
use gdtf_content_families::FieldsFamily;
use load_suite::suite::{self, FamilyLoadContract};

impl FamilyLoadContract for FieldsFamily {
        const EXPECTED_MEMBERS: &'static [&'static str] = &["toxic_waste_pool"];

    fn is_empty(registry: &FieldDefRegistry) -> bool {
        registry.is_empty()
    }

    fn member_resolves(registry: &FieldDefRegistry, label: &str) -> bool {
        registry.def(&FieldKey::new(label.to_owned())).is_some()
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
