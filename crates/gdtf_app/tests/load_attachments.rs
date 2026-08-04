//! Load attachments into the attachments family by authored member keys.
//! Value-agnostic: registry presence and stems only.
mod load_suite;

#[path = "load_suite/behaviors.rs"]
mod behaviors;

use behaviors::FamilyBehaviorContract;
use gdtf_battle_sim::{
    equipment::attachments::{AttachmentName, AttachmentRegistry, AttachmentSpec},
    weapon::WeaponName,
};
use gdtf_content_families::AttachmentsFamily;
use load_suite::suite::{self, FamilyLoadContract};

const REDRIVE_SENTINEL: &str = "Redrive Sentinel";

impl FamilyLoadContract for AttachmentsFamily {
    fn is_empty(registry: &AttachmentRegistry) -> bool {
        registry.is_empty()
    }
}

impl FamilyBehaviorContract for AttachmentsFamily {
    const SALVAGE_FIXTURE_ROOT: &'static str = "attachment_salvage_root";

    const SALVAGE_GOOD_MEMBERS: &'static [&'static str] = &["good_optic", "good_grip"];

    const SALVAGE_BROKEN_FILE: &'static str = "broken.attachment.ron";

    const MISSING_FOLDER_FIXTURE_ROOT: &'static str = "missing_family_root";

    const PROBE_MEMBER: &'static str = "scoped_sight";

    fn is_empty(registry: &AttachmentRegistry) -> bool {
        registry.is_empty()
    }

    fn member_resolves(registry: &AttachmentRegistry, label: &str) -> bool {
        registry
            .spec(&AttachmentName::new(label.to_owned()))
            .is_some()
    }

    fn mutate_spec(spec: &mut AttachmentSpec) {
        spec.display_name = WeaponName::new(REDRIVE_SENTINEL.to_owned());
    }

    fn mutation_visible(registry: &AttachmentRegistry, label: &str) -> bool {
        registry
            .spec(&AttachmentName::new(label.to_owned()))
            .is_some_and(|spec| *spec.display_name == *REDRIVE_SENTINEL)
    }
}

#[test]
fn attachments_loader_no_ops_cleanly_without_asset_server() {
    suite::loader_no_ops_without_asset_server::<AttachmentsFamily>();
}

#[test]
fn load_does_not_leave_without_an_attachment_registry() {
    suite::load_gates_on_registry::<AttachmentsFamily>();
}

#[test]
fn real_asset_resolves_attachment_registry_keyed_by_filename() {
    suite::real_asset_resolves_registry::<AttachmentsFamily>();
}

#[test]
fn malformed_attachment_is_salvaged_around_and_reported() {
    behaviors::salvage_parity::<AttachmentsFamily>();
}

#[test]
fn missing_attachments_folder_fails_closed_to_empty_registry() {
    behaviors::missing_folder_fails_closed_empty::<AttachmentsFamily>();
}

#[test]
fn attachment_registry_is_never_published_partial() {
    behaviors::never_publishes_partial::<AttachmentsFamily>();
}

#[test]
fn modified_attachment_member_rebuilds_registry_live() {
    behaviors::redrive_rebuilds_live::<AttachmentsFamily>();
}
