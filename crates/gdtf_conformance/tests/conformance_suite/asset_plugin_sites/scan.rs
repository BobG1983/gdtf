//! Reading one file for asset plugins that leave the file watcher on.

use std::{fs, path::Path};

// Assembled where they are used, so this file never matches its own needles.
const TYPE_NAME: &str = "AssetPlugin";

const OVERRIDE_OFF: &str = "watch_for_changes_override: Some(false)";

/// What one file's asset plugins say about the watcher.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Verdict {
    /// The file builds no asset plugin, or every one it builds turns the watcher off.
    Unwatched,
    /// The file builds at least one asset plugin that watches its root.
    Watches,
}

/// Read `path` and say whether its asset plugins turn the watcher off. A file that is
/// not UTF-8 builds nothing.
pub(crate) fn verdict_for(path: &Path) -> Verdict {
    fs::read_to_string(path).map_or(Verdict::Unwatched, |text| verdict(&text))
}

fn verdict(text: &str) -> Verdict {
    let literals = literal_spans(text);
    let watched = literals
        .iter()
        .any(|&(start, end)| !text[start..end].contains(OVERRIDE_OFF));
    if watched {
        return Verdict::Watches;
    }
    if default_call_outside(text, &literals) {
        return Verdict::Watches;
    }
    Verdict::Unwatched
}

// Byte range of every struct literal of the plugin type, from its opening brace to the
// matching closing one. An unbalanced literal runs to the end of the file, which leaves
// it without the override and so fails.
fn literal_spans(text: &str) -> Vec<(usize, usize)> {
    let opener = format!("{TYPE_NAME} {{");
    let mut spans = Vec::new();
    let mut from = 0;
    while let Some(found) = text[from..].find(&opener) {
        let name = from + found;
        let open = name + opener.len() - 1;
        if opens_a_literal(text, name) {
            let end = matching_brace(text, open).unwrap_or(text.len());
            spans.push((open, end));
        }
        from = open + 1;
    }
    spans
}

// An arrow before the type name makes it a return type and the brace a function body.
// A `for` before it makes the brace a trait impl. Neither one builds a plugin.
fn opens_a_literal(text: &str, name: usize) -> bool {
    let before = text[..name].trim_end();
    !before.ends_with("->") && !before.ends_with(" for")
}

fn matching_brace(text: &str, open: usize) -> Option<usize> {
    let mut depth = 0_usize;
    for (offset, ch) in text[open..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Some(open + offset);
                }
            }
            _ => {}
        }
    }
    None
}

// A fully defaulted plugin watches. One written as the tail of a literal is already
// covered by that literal's own check.
fn default_call_outside(text: &str, literals: &[(usize, usize)]) -> bool {
    let call = format!("{TYPE_NAME}::default()");
    let mut from = 0;
    while let Some(found) = text[from..].find(&call) {
        let at = from + found;
        let inside = literals.iter().any(|&(start, end)| at > start && at < end);
        if !inside {
            return true;
        }
        from = at + call.len();
    }
    false
}
