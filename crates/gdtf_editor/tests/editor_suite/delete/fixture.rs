//! The temp assets root every case in this suite loads and deletes from.

use std::path::{Path, PathBuf};

use bevy::asset::uuid::Uuid;
use gdtf_battle_sim::{
    armor::InjuryCategory,
    ganger::{GangMember, GangName, GangRoster},
    injuries::{DamageContext, InjuryName, InjuryTables, InjuryWeight, WeightedInjuryEntry},
    level::{GridHeight, GridLevels, GridSize, GridWidth, PrefabSpec, SpawnRole, ThemeUuid},
    severity::Severity,
    weapon::WeaponName,
};
use gdtf_content_families::prefabs::{PREFAB_EXTENSION, PREFABS_FOLDER};
use gdtf_editor::{
    GangDraft, InjuryDraft, WeaponDraft, WeightingDraft, draft_to_def, draft_to_roster,
    draft_to_weapon_spec, draft_to_weighting, gang_save_path_in, write_gang_in, write_injury_in,
    write_weapon_in, write_weighting_in,
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
        member.weapon = Some(WeaponName::new(weapon.to_owned()));
    }
    let (name, roster) = draft_to_roster(&draft);
    write_gang_in(root, &name, &roster).is_ok()
}

/// Write a one-member gang under `root`, `equip` setting that member's loadout.
pub(crate) fn write_gang_equipped(root: &Path, equip: impl Fn(&mut GangMember)) -> bool {
    let mut draft = GangDraft::new_gang();
    draft.set_name(FIXTURE_GANG.to_owned());
    draft.add_member();
    if let Some(member) = draft.members_mut().first_mut() {
        equip(member);
    }
    let (name, roster) = draft_to_roster(&draft);
    write_gang_in(root, &name, &roster).is_ok()
}

/// The file the fixture gang is written to under `root`.
#[must_use]
pub(crate) fn fixture_gang_path(root: &Path) -> PathBuf {
    gang_save_path_in(root, &GangName::new(FIXTURE_GANG.to_owned()))
}

/// The fixture gang's one member, as the file under `root` holds it.
#[must_use]
pub(crate) fn fixture_gang_member(root: &Path) -> Option<GangMember> {
    let ron = std::fs::read_to_string(fixture_gang_path(root)).ok()?;
    let roster: GangRoster = ron::from_str(&ron).ok()?;
    roster.members.first().cloned()
}

/// A weapon name from a file stem.
#[must_use]
pub(crate) fn weapon_name(stem: &str) -> WeaponName {
    WeaponName::new(stem.to_owned())
}

/// The prefab this suite deletes, and the theme its spec names.
pub(crate) const FIXTURE_PREFAB: &str = "fixture_prefab";

/// The theme the fixture prefab belongs to; no def in a temp root resolves it.
const FIXTURE_THEME: &str = "00000000-0000-0000-0000-063000000a01";

/// The 3x3x1 fixture prefab's spec, the one its file authors.
#[must_use]
pub(crate) fn prefab_spec() -> PrefabSpec {
    let Ok(uuid) = Uuid::parse_str(FIXTURE_THEME) else {
        return PrefabSpec::new(
            ThemeUuid::nil(),
            GridSize::default(),
            SpawnRole::Fill,
            Vec::new(),
        );
    };
    PrefabSpec::new(
        ThemeUuid::new(uuid),
        fixture_grid(),
        SpawnRole::Fill,
        Vec::new(),
    )
}

// The 3x3x1 extent both the file and the spec name.
fn fixture_grid() -> GridSize {
    GridSize::new(GridWidth::new(3), GridHeight::new(3), GridLevels::new(1)).unwrap_or_default()
}

/// Write the fixture prefab under `root`, answering the file it wrote.
pub(crate) fn write_fixture_prefab(root: &Path) -> Option<PathBuf> {
    let path = root
        .join(PREFABS_FOLDER)
        .join("fixture_theme")
        .join("3x3")
        .join(format!("{FIXTURE_PREFAB}.{PREFAB_EXTENSION}"));
    std::fs::create_dir_all(path.parent()?).ok()?;
    let ron = format!(
        "(theme: \"{FIXTURE_THEME}\", size: (width: 3, height: 3, levels: 1), placements: [])",
    );
    std::fs::write(&path, ron).ok()?;
    Some(path)
}

/// The one injury the fixture weighting's Minor row resolves to.
pub(crate) const WEIGHTED_INJURY: &str = "fixture_bruise";

/// Write an injury def under `root` in `category`, answering whether the write succeeded.
pub(crate) fn write_fixture_injury(root: &Path, category: InjuryCategory) -> bool {
    let mut draft = InjuryDraft::new_injury();
    draft.set_key(WEIGHTED_INJURY.to_owned());
    let (name, mut def) = draft_to_def(&draft);
    def.category = category;
    def.severity = Severity::Minor;
    write_injury_in(root, &name, &def).is_ok()
}

/// Write a weighting table under `root` whose one Minor row names [`WEIGHTED_INJURY`].
pub(crate) fn write_fixture_weighting(
    root: &Path,
    category: InjuryCategory,
    context: DamageContext,
) -> bool {
    let mut draft = WeightingDraft::default();
    draft.load_table(category, context, &InjuryTables::default());
    draft.weighting_mut().minor.push(WeightedInjuryEntry::new(
        InjuryName::new(WEIGHTED_INJURY.to_owned()),
        InjuryWeight::new(2),
    ));
    write_weighting_in(root, &draft_to_weighting(&draft)).is_ok()
}
