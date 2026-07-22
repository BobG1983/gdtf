//! GTW-549 / GTW-619: the ATTACHMENTS family's load coverage — the thin
//! wrapper over the generic per-family suite (`load_suite::suite`) PLUS the
//! GTW-619 deep-behavior extension (`load_suite/behaviors.rs`, included
//! standalone), proving the migration off the bespoke resolve/redrive chain
//! onto the GTW-570 generic content-family loader preserved every load behavior:
//! salvage parity, fail-closed-empty, never-publish-partial, and the live
//! redrive. The family-bespoke shipped weapon-key / slot-fit pins live in
//! `load_attachment_fit.rs`.
//!
//! VALUE-AGNOSTIC: registry presence + authored filename-stem keys + an
//! in-memory sentinel edit — never a shipped magnitude (the brittle-test
//! rule).

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

/// The sentinel display name the redrive walk edits into the probe member's
/// in-memory spec — never a shipped value, so the pin stays value-agnostic.
const REDRIVE_SENTINEL: &str = "GTW-619 Redrive Sentinel";

impl FamilyLoadContract for AttachmentsFamily {
    /// Shipped stems the two shipped weapons author (`scoped_sight` on
    /// `las_carbine`, `suppressor` on `stub_pistol`) — the keys that must
    /// never resolve to nothing at runtime.
    const EXPECTED_MEMBERS: &'static [&'static str] = &["scoped_sight", "suppressor"];

    fn is_empty(registry: &AttachmentRegistry) -> bool {
        registry.is_empty()
    }

    fn member_resolves(registry: &AttachmentRegistry, label: &str) -> bool {
        registry
            .spec(&AttachmentName::new(label.to_owned()))
            .is_some()
    }
}

impl FamilyBehaviorContract for AttachmentsFamily {
    /// `content/attachments/` with 2 well-formed items + 1 deliberately
    /// malformed `broken.attachment.ron`.
    const SALVAGE_FIXTURE_ROOT: &'static str = "attachment_salvage_root";

    /// Both well-formed fixture siblings must survive the per-file salvage.
    const SALVAGE_GOOD_MEMBERS: &'static [&'static str] = &["good_optic", "good_grip"];

    /// The malformed fixture member the report must name.
    const SALVAGE_BROKEN_FILE: &'static str = "broken.attachment.ron";

    /// The shared family-agnostic root that materializes NO content folders —
    /// `content/attachments` is unenumerable there, the genuine-`Failed` path.
    const MISSING_FOLDER_FIXTURE_ROOT: &'static str = "missing_family_root";

    /// A shipped member (`content/attachments/scoped_sight.attachment.ron`)
    /// the partial-hold and redrive walks address by path.
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
        // Value-agnostic sentinel: the display label, never an effect magnitude.
        spec.display_name = WeaponName::new(REDRIVE_SENTINEL.to_owned());
    }

    fn mutation_visible(registry: &AttachmentRegistry, label: &str) -> bool {
        registry
            .spec(&AttachmentName::new(label.to_owned()))
            .is_some_and(|spec| *spec.display_name == *REDRIVE_SENTINEL)
    }
}

/// Tier (a) — the attachments loader registration + folder kick-off no-op
/// cleanly under `MinimalPlugins` (bevy-traps rule 1).
#[test]
fn attachments_loader_no_ops_cleanly_without_asset_server() {
    suite::loader_no_ops_without_asset_server::<AttachmentsFamily>();
}

/// Tier (a) companion — the Load→Intro transition GATES on the
/// [`AttachmentRegistry`] (the GTW-549 gate clause: the attachments folder is
/// verified loaded before any battle resolves a weapon's attachment keys).
#[test]
fn load_does_not_leave_without_an_attachment_registry() {
    suite::load_gates_on_registry::<AttachmentsFamily>();
}

/// Tier (b) — the REAL `assets/content/attachments/` folder resolves into a
/// stem-keyed [`AttachmentRegistry`] through the GTW-619 generic content-family
/// registration in the Load plugin.
#[test]
fn real_asset_resolves_attachment_registry_keyed_by_filename() {
    suite::real_asset_resolves_registry::<AttachmentsFamily>();
}

/// GTW-619 deep behavior — one malformed member no longer empties the family:
/// the per-file salvage folds both well-formed siblings, reports the broken
/// file loudly, and Load still exits (parity with the deleted bespoke chain).
#[test]
fn malformed_attachment_is_salvaged_around_and_reported() {
    behaviors::salvage_parity::<AttachmentsFamily>();
}

/// GTW-619 deep behavior — a missing `content/attachments/` folder fails
/// closed to the EMPTY registry (a weapon's authored key then resolves to
/// nothing rather than stranding Load).
#[test]
fn missing_attachments_folder_fails_closed_to_empty_registry() {
    behaviors::missing_folder_fails_closed_empty::<AttachmentsFamily>();
}

/// GTW-619 deep behavior — the resolve never publishes a partial registry
/// while a member is absent from its collection, and publishes the FULL
/// registry once it returns.
#[test]
fn attachment_registry_is_never_published_partial() {
    behaviors::never_publishes_partial::<AttachmentsFamily>();
}

/// GTW-619 deep behavior — a Modified member rebuilds the resident registry
/// in place (the GTW-549 live hot-reload, now through the generic redrive).
#[test]
fn modified_attachment_member_rebuilds_registry_live() {
    behaviors::redrive_rebuilds_live::<AttachmentsFamily>();
}
