//! The temp assets root every case in this suite loads and deletes from.

use std::path::Path;

use gdtf_battle_sim::weapon::WeaponName;
use gdtf_content_editor::{
    GangDraft, WeaponDraft, draft_to_roster, draft_to_weapon_spec, write_gang_in, write_weapon_in,
};

/// The weapon the fixture gang's one member names.
pub(crate) const FIXTURE_GUN: &str = "fixture_gun";

/// A weapon no record references.
pub(crate) const ORPHAN_GUN: &str = "orphan_gun";

/// The gang whose member holds the reference to [`FIXTURE_GUN`].
pub(crate) const FIXTURE_GANG: &str = "fixture_gang";

/// Write a weapon under `root`, answering whether the write succeeded.
pub(crate) fn write_fixture_weapon(root: &Path, stem: &str) -> bool {
    let mut draft = WeaponDraft::new_weapon();
    draft.set_name(stem.to_owned());
    let (name, spec) = draft_to_weapon_spec(&draft);
    write_weapon_in(root, &name, &spec).is_ok()
}

/// Write a one-member gang under `root` whose member names `weapon`.
pub(crate) fn write_fixture_gang(root: &Path, weapon: &str) -> bool {
    let mut draft = GangDraft::new_gang();
    draft.set_name(FIXTURE_GANG.to_owned());
    draft.add_member();
    if let Some(member) = draft.members_mut().first_mut() {
        member.weapon = WeaponName::new(weapon.to_owned());
    }
    let (name, roster) = draft_to_roster(&draft);
    write_gang_in(root, &name, &roster).is_ok()
}

/// A weapon name from a file stem.
#[must_use]
pub(crate) fn weapon_name(stem: &str) -> WeaponName {
    WeaponName::new(stem.to_owned())
}
