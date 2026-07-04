//! GTW-545 / GTW-580: the area-damage-fields family's load coverage — the thin
//! wrapper over the generic per-family suite (`load_suite::suite`).
//!
//! The tier structure (`MinimalPlugins` no-op guard + gate pin, headless
//! real-asset folder resolve) is encoded ONCE in the shared suite; this file
//! only binds it to [`FieldsFamily`] with the authored member key. The
//! assertions stay VALUE-AGNOSTIC (catalog presence + the authored
//! filename-stem key) — the per-round drain / immunity / seed MECHANISM is
//! covered by the in-crate sim tests + the `gtw545_fields` integration test.

mod load_suite;

use gdtf_battle_sim::{FieldDefRegistry, FieldKey};
use gdtf_content_families::FieldsFamily;
use load_suite::suite::{self, FamilyLoadContract};

impl FamilyLoadContract for FieldsFamily {
    /// The canonical shipped stem: `toxic_waste_pool.field.ron`.
    const EXPECTED_MEMBERS: &'static [&'static str] = &["toxic_waste_pool"];

    fn is_empty(registry: &FieldDefRegistry) -> bool {
        registry.is_empty()
    }

    fn member_resolves(registry: &FieldDefRegistry, label: &str) -> bool {
        registry.def(&FieldKey::new(label.to_owned())).is_some()
    }
}

/// AC (tier a) — the field loader registration + folder kick-off no-op cleanly
/// under `MinimalPlugins` (bevy-traps rule 1).
#[test]
fn field_loader_no_ops_cleanly_without_asset_server() {
    suite::loader_no_ops_without_asset_server::<FieldsFamily>();
}

/// AC (companion, tier a) — the Load→Intro transition GATES on the
/// [`FieldDefRegistry`] (the GTW-545 gate clause: the fields catalog is
/// verified loaded before `Load` exits, else a battle seeding a field could
/// hit `FieldNotFound` at setup).
#[test]
fn load_does_not_leave_without_a_field_def_registry() {
    suite::load_gates_on_registry::<FieldsFamily>();
}

/// AC (tier b) — the REAL `assets/content/fields/` folder resolves into a
/// stem-keyed [`FieldDefRegistry`] through the Load code path.
#[test]
fn real_asset_resolves_field_def_registry_keyed_by_filename() {
    suite::real_asset_resolves_registry::<FieldsFamily>();
}
