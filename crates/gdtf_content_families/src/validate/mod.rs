//! The **host-agnostic per-edge reference checks** of the GTW-582 unified
//! dangling-reference contract (moved here from the game's `Load` validator in
//! GTW-630, so BOTH hosts — the game app and the content editor — register the
//! SAME check over the same family glue, and a dangling key authored in the
//! editor surfaces at authoring time, not on the next game launch).
//!
//! Each check is one Bevy system over sim registries: it walks ONE edge family
//! of the authored content graph and appends a typed
//! [`ContentFinding`](gdtf_assets::ContentFinding) to the
//! [`ContentIntegrityReport`](gdtf_assets::ContentIntegrityReport) for every
//! reference that resolves nothing. A host registers a check with one
//! [`register_reference_check`](gdtf_assets::ContentValidationAppExt::register_reference_check)
//! hook and gates
//! [`ContentValidationSet::Check`](gdtf_assets::ContentValidationSet) on its
//! own "every registry my registered checks read is present, not yet checked"
//! window — the checks take plain `Res<…>` on that contract (`bevy-traps.md`
//! #1 is guarded ONCE, at the host's set condition). Register only checks
//! whose registries the host actually loads: an absent registry would fail the
//! window (or, unguarded, fail param validation) — e.g. the editor registers
//! the theme→terrain, emplacement→weapon, and gang-equipment edges but NOT
//! weapons→attachments, because it never loads the attachment items.
//!
//! Submodules by EDGE FAMILY (wiring only here; the game-bespoke edges —
//! situation, prefabs, injuries — stay in the game's
//! `states/load/systems/validate/`, the adopted GTW-630 split):
//!
//! - [`check_gang_equipment_refs`] — every roster member's equipment keys
//!   (weapon / armor / melee, incl. the implicit `fists` default).
//! - [`check_weapon_attachment_refs`] — every ranged + melee weapon's
//!   attachment keys.
//! - [`check_theme_terrain_refs`] / [`check_emplacement_weapon_refs`] —
//!   theme → terrain-def UUIDs and emplacement → mounted-weapon keys.

mod attachments;
mod gangs;
mod terrain;

pub use attachments::check_weapon_attachment_refs;
pub use gangs::check_gang_equipment_refs;
pub use terrain::{check_emplacement_weapon_refs, check_theme_terrain_refs};
