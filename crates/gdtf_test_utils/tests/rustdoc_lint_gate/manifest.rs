//! The small TOML-subset manifest reader this guard needs — the non-comment
//! lines inside one named section.
//!
//! Deliberately NOT a `toml` dependency: the guard reads flat, hand-kept
//! manifests written in one style (`key = value` on one line, whole-line
//! comments), and the sibling guards in this crate are std-only for the same
//! reason.
//!
//! A declaration re-spelled across multiple lines falls outside that style and
//! is reported MISSING rather than silently accepted — a loud false alarm, and
//! the fix is to write it on one line as every manifest here does today.

/// Collapse a line's runs of whitespace to single spaces, so indentation and
/// padding never decide a comparison: `all  =  { level = "deny" }` reads as
/// `all = { level = "deny" }`.
fn normalize(line: &str) -> String {
    line.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The non-comment, non-empty, whitespace-normalized lines inside `[section]`.
///
/// A section ends at the next `[header]` line. `section` is the header WITHOUT
/// its brackets for a table (`workspace.lints.rustdoc`) and WITH the inner pair
/// for an array-of-tables (`[bin]`, matching a `[[bin]]` header). Only
/// whole-line comments are stripped; the manifests this guard reads put every
/// comment on its own line.
pub(crate) fn section_lines(manifest: &str, section: &str) -> Vec<String> {
    let header = format!("[{section}]");
    let mut inside = false;
    let mut lines = Vec::new();
    for raw in manifest.lines() {
        let trimmed = raw.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            inside = trimmed == header;
            continue;
        }
        if !inside || trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        lines.push(normalize(trimmed));
    }
    lines
}

/// Whether `[section]` declares exactly `declaration` (whitespace-normalized).
pub(crate) fn declares(manifest: &str, section: &str, declaration: &str) -> bool {
    let wanted = normalize(declaration);
    section_lines(manifest, section).contains(&wanted)
}
