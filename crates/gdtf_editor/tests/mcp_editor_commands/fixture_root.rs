//! The records this suite's delete cases write into a temp assets root and then remove.

use std::path::{Path, PathBuf};

use bevy::asset::uuid::Uuid;
use gdtf_assets::ContentFileStem;
use gdtf_battle_sim::{
    armor::InjuryCategory,
    injuries::{DamageContext, InjuryName, InjuryTables, InjuryWeight, WeightedInjuryEntry},
    level::{GridHeight, GridLevels, GridSize, GridWidth, PrefabSpec, SpawnRole, ThemeUuid},
    severity::Severity,
};
use gdtf_content_families::prefabs::{PREFAB_EXTENSION, PREFABS_FOLDER, member_key};
use gdtf_editor::{
    InjuryDraft, WeightingDraft, draft_to_def, draft_to_weighting, write_injury_in,
    write_weighting_in,
};

/// The prefab this suite deletes over the wire.
pub(crate) const FIXTURE_PREFAB: &str = "fixture_prefab";

/// The terrain piece the delete cases remove.
pub(crate) const DELETED_PIECE: &str = "00000000-0000-0000-0000-133100000001";

/// The terrain piece the delete cases point the theme at.
pub(crate) const REPLACEMENT_PIECE: &str = "00000000-0000-0000-0000-133100000002";

/// The theme whose `default_floor` names [`DELETED_PIECE`].
pub(crate) const FLOOR_THEME: &str = "00000000-0000-0000-0000-1331000000a1";

// The folder terrain defs and theme defs are read from.
const TERRAIN_FOLDER: &str = "content/terrain";

/// Write a cover terrain def under `root`, answering the file it wrote.
pub(crate) fn write_terrain_def(root: &Path, stem: &str, uuid: &str) -> Option<PathBuf> {
    let body = format!(
        "(
    key: \"{uuid}\",
    display_name: \"Piece Named Unlike {stem}\",
    sim_kind: Cover(hp: 10, armor_protection: 0, armor_hardness: 0, height_band: Low),
    presenter_kind: Cover,
    views: [
        (view: Facing(North), sprite: \"cover\"),
        (view: Facing(East), sprite: \"cover\"),
        (view: Facing(South), sprite: \"cover\"),
        (view: Facing(West), sprite: \"cover\"),
    ],
    tags: [],
)
"
    );
    write_under(
        root,
        &format!("{TERRAIN_FOLDER}/{stem}.terrain_def.ron"),
        &body,
    )
}

/// Write a theme def under `root` whose default floor is `default_floor`.
pub(crate) fn write_theme_def(
    root: &Path,
    stem: &str,
    uuid: &str,
    default_floor: &str,
    palette: &str,
) -> Option<PathBuf> {
    let body = format!(
        "(
    key: \"{uuid}\",
    display_name: \"Theme Named Unlike {stem}\",
    default_floor: \"{default_floor}\",
    terrain: [\"{palette}\"],
)
"
    );
    write_under(
        root,
        &format!("{TERRAIN_FOLDER}/{stem}.terrain_theme.ron"),
        &body,
    )
}

// Write one file under `root`, making its folder first.
fn write_under(root: &Path, relative: &str, body: &str) -> Option<PathBuf> {
    let path = root.join(relative);
    std::fs::create_dir_all(path.parent()?).ok()?;
    std::fs::write(&path, body).ok()?;
    Some(path)
}

/// The one injury the fixture weighting's Minor row resolves to.
pub(crate) const WEIGHTED_INJURY: &str = "fixture_bruise";

// The theme the fixture prefab belongs to; no def in a temp root resolves it.
const FIXTURE_THEME: &str = "00000000-0000-0000-0000-063000000a01";

/// The key the fixture prefab is recorded under, as `editor.delete_record` takes it.
#[must_use]
pub(crate) fn prefab_member_key() -> String {
    (*member_key(
        &ContentFileStem::new(FIXTURE_PREFAB.to_owned()),
        &prefab_spec(),
    ))
    .clone()
}

// The spec the fixture prefab's file authors.
fn prefab_spec() -> PrefabSpec {
    let theme = Uuid::parse_str(FIXTURE_THEME).map_or_else(|_| ThemeUuid::nil(), ThemeUuid::new);
    PrefabSpec::new(theme, fixture_grid(), SpawnRole::Fill, Vec::new())
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
