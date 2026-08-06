use std::{fs, path::Path};

use super::tree::repo_root;

// Where each command declares the name it publishes under.
const COMMAND_SOURCES: &str = "crates/gdtf_app/src/dev/net_qa/commands";

// The literal a command's name is declared with.
const NAME_DECLARATION: &str = "CommandName::from_static(\"";

// What a dotted word ends with when it is a file name and not a command.
const FILE_ENDINGS: &[&str] = &["rs", "md", "ron", "toml", "json", "png"];

// Every name a command declares itself with, read out of the command sources.
pub(crate) fn published_command_names() -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    collect_names(&repo_root().join(COMMAND_SOURCES), &mut names);
    names.sort_unstable();
    names.dedup();
    names
}

fn collect_names(directory: &Path, names: &mut Vec<String>) {
    let Ok(entries) = fs::read_dir(directory) else {
        unreachable!("{} must be readable", directory.display());
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if is_test_path(&path) {
            continue;
        }
        if path.is_dir() {
            collect_names(&path, names);
        } else if path.extension().is_some_and(|kind| kind == "rs")
            && let Ok(source) = fs::read_to_string(&path)
        {
            names.extend(names_in(&source));
        }
    }
}

// Test modules declare fake commands the same way real ones do, and no host publishes them.
fn is_test_path(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| {
            matches!(name, "test" | "tests" | "test.rs" | "tests.rs") || name.starts_with("test_")
        })
}

fn names_in(source: &str) -> Vec<String> {
    source
        .split(NAME_DECLARATION)
        .skip(1)
        .filter_map(|tail| tail.split('"').next())
        .map(str::to_owned)
        .collect()
}

// The part before the dot of every published name — the families the game has.
pub(crate) fn published_families(published: &[String]) -> Vec<&str> {
    let mut families: Vec<&str> = published
        .iter()
        .filter_map(|name| name.split_once('.'))
        .map(|(family, _)| family)
        .collect();
    families.sort_unstable();
    families.dedup();
    families
}

// True when a backticked word is written the way a command name is written: `family.verb`,
// lowercase and dotted once, not a file name, in a family the game already publishes.
pub(crate) fn reads_as_command_name(word: &str, families: &[&str]) -> bool {
    let Some((family, verb)) = word.split_once('.') else {
        return false;
    };
    !family.is_empty()
        && !verb.is_empty()
        && !verb.contains('.')
        && !FILE_ENDINGS.contains(&verb)
        && word.chars().all(is_name_letter)
        && families.contains(&family)
}

// What a command name is spelled out of.
const fn is_name_letter(letter: char) -> bool {
    letter.is_ascii_lowercase() || letter.is_ascii_digit() || letter == '_' || letter == '.'
}
