//! The terrain, theme, prefab and situation records this suite's replace cases author.

use std::{
    fmt::Write as _,
    path::{Path, PathBuf},
};

use gdtf_content_families::{
    prefabs::{PREFAB_EXTENSION, PREFABS_FOLDER},
    situation::SITUATION_RON_PATH,
};

/// The folder every terrain def and theme def is read from.
pub(crate) const TERRAIN_FOLDER: &str = "content/terrain";

/// The terrain piece a replace case deletes.
pub(crate) const DELETED_PIECE: &str = "00000000-0000-0000-0000-133000000001";

/// The terrain piece a replace case points every referrer at.
pub(crate) const REPLACEMENT_PIECE: &str = "00000000-0000-0000-0000-133000000002";

/// A third terrain piece, so an offer holds more than one row.
pub(crate) const SPARE_PIECE: &str = "00000000-0000-0000-0000-133000000003";

/// The theme a replace case deletes.
pub(crate) const DELETED_THEME: &str = "00000000-0000-0000-0000-1330000000a1";

/// The theme a replace case points every referrer at.
pub(crate) const REPLACEMENT_THEME: &str = "00000000-0000-0000-0000-1330000000a2";

// A cover def's four views, all naming one sprite key.
const COVER_VIEWS: &str = "views: [
        (view: Facing(North), sprite: \"cover\"),
        (view: Facing(East), sprite: \"cover\"),
        (view: Facing(South), sprite: \"cover\"),
        (view: Facing(West), sprite: \"cover\"),
    ],";

/// Write one file under `root`, making its folder first; answers the file it wrote.
pub(crate) fn write_under(root: &Path, relative: &str, body: &str) -> Option<PathBuf> {
    let path = root.join(relative);
    std::fs::create_dir_all(path.parent()?).ok()?;
    std::fs::write(&path, body).ok()?;
    Some(path)
}

/// Write a cover terrain def whose display name does not match its file stem.
pub(crate) fn write_terrain_def(root: &Path, stem: &str, uuid: &str) -> Option<PathBuf> {
    write_terrain_def_with(root, stem, uuid, "", "")
}

/// The display name `write_terrain_def` gives the def it writes under `stem`.
pub(crate) fn terrain_display_name(stem: &str) -> String {
    format!("Piece Named Unlike {stem}")
}

/// Write a cover terrain def whose `leaves_behind` names another piece.
pub(crate) fn write_leaves_behind_def(
    root: &Path,
    stem: &str,
    uuid: &str,
    successor: &str,
) -> Option<PathBuf> {
    write_terrain_def_with(
        root,
        stem,
        uuid,
        "",
        &format!("leaves_behind: Piece(\"{successor}\"),"),
    )
}

/// Write an emplacement terrain def mounting `weapon`.
pub(crate) fn write_emplacement_def(
    root: &Path,
    stem: &str,
    uuid: &str,
    weapon: &str,
) -> Option<PathBuf> {
    let sim_kind = format!(
        "sim_kind: Emplacement(
        hp:               10,
        armor_protection: 0,
        armor_hardness:   0,
        height_band:      Low,
        mounted_weapon:   \"{weapon}\",
    ),
    presenter_kind: Emplacement,"
    );
    write_terrain_def_with(root, stem, uuid, &sim_kind, "")
}

/// Write a theme def whose display name does not match its file stem.
pub(crate) fn write_theme_def(
    root: &Path,
    stem: &str,
    uuid: &str,
    default_floor: &str,
    palette: &[&str],
) -> Option<PathBuf> {
    let terrain = palette.iter().fold(String::new(), |mut rows, piece| {
        let _ = writeln!(rows, "        \"{piece}\",");
        rows
    });
    let body = format!(
        "(
    key: \"{uuid}\",
    display_name: \"Theme Named Unlike {stem}\",
    default_floor: \"{default_floor}\",
    terrain: [
{terrain}    ],
)
"
    );
    write_under(
        root,
        &format!("{TERRAIN_FOLDER}/{stem}.terrain_theme.ron"),
        &body,
    )
}

/// Write a 3x3 prefab under `folder`, placing every piece named, one per cell.
pub(crate) fn write_prefab(
    root: &Path,
    folder: &str,
    stem: &str,
    theme: &str,
    pieces: &[&str],
) -> Option<PathBuf> {
    let placements = pieces
        .iter()
        .enumerate()
        .fold(String::new(), |mut rows, (at, piece)| {
            let _ = writeln!(
                rows,
                "        (piece: \"{piece}\", at: (cell: (x: {at}, y: 0), level: 0)),"
            );
            rows
        });
    let body = format!(
        "(theme: \"{theme}\", size: (width: 3, height: 3, levels: 1), placements: [
{placements}    ])
"
    );
    write_under(
        root,
        &format!("{PREFABS_FOLDER}/{folder}/3x3/{stem}.{PREFAB_EXTENSION}"),
        &body,
    )
}

/// Write the authored situation under `root`, from the body a case names.
pub(crate) fn write_situation(root: &Path, body: &str) -> Option<PathBuf> {
    write_under(root, SITUATION_RON_PATH, body)
}

/// The situation file under `root`.
#[must_use]
pub(crate) fn situation_path(root: &Path) -> PathBuf {
    root.join(SITUATION_RON_PATH)
}

/// Every file under `root`, sorted, with the bytes it held.
#[must_use]
pub(crate) fn file_snapshot(root: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    let mut files: Vec<(PathBuf, Vec<u8>)> = Vec::new();
    collect_files(root, &mut files);
    files.sort_by(|left, right| left.0.cmp(&right.0));
    files
}

/// Whether any file under `root` still holds `text`.
#[must_use]
pub(crate) fn root_holds(root: &Path, text: &str) -> bool {
    file_snapshot(root)
        .iter()
        .any(|(_path, bytes)| String::from_utf8_lossy(bytes).contains(text))
}

/// The text of one file, or the empty string when it is not there.
#[must_use]
pub(crate) fn file_text(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_default()
}

// Every file under `at`, walked depth first.
fn collect_files(at: &Path, into: &mut Vec<(PathBuf, Vec<u8>)>) {
    let Ok(entries) = std::fs::read_dir(at) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_files(&path, into);
        } else if let Ok(bytes) = std::fs::read(&path) {
            into.push((path, bytes));
        }
    }
}

// A cover terrain def, with an optional sim kind override and one extra row.
fn write_terrain_def_with(
    root: &Path,
    stem: &str,
    uuid: &str,
    sim_kind: &str,
    extra: &str,
) -> Option<PathBuf> {
    let kind = if sim_kind.is_empty() {
        "sim_kind: Cover(
        hp:               10,
        armor_protection: 0,
        armor_hardness:   0,
        height_band:      Low,
    ),
    presenter_kind: Cover,"
            .to_owned()
    } else {
        sim_kind.to_owned()
    };
    let display_name = terrain_display_name(stem);
    let body = format!(
        "(
    key: \"{uuid}\",
    display_name: \"{display_name}\",
    {kind}
    {COVER_VIEWS}
    tags: [],
    {extra}
)
"
    );
    write_under(
        root,
        &format!("{TERRAIN_FOLDER}/{stem}.terrain_def.ron"),
        &body,
    )
}
