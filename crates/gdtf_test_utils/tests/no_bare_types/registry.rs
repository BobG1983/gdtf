//! The exemption registry — `.claude/rules/no-bare-types-exemptions.txt`,
//! mirroring `module-layout-exemptions.txt`. One entry per non-comment line:
//! `<path:line:col> <type> — <why> (approved/tracked: GTW-N, <date>)`. The
//! first two whitespace tokens (`<path:line:col>` and `<type>`) form the key
//! matched against [`crate::types::Violation::key`].
//!
//! Two kinds of entry live here, distinguished only by their `<why>` prose:
//! documented FALSE POSITIVES (a checker limitation, permanent) and KNOWN
//! GENUINE violations pending a follow-up ticket (removed when that ticket
//! fixes the source). The conformance test suppresses both and FAILS on any
//! STALE entry (a key that no longer corresponds to a live violation), so the
//! registry only ever shrinks or is deliberately re-approved.

use std::{collections::BTreeSet, path::Path};

/// The parsed registry — the suppression keys plus the raw entry lines (for
/// stale-entry reporting).
pub(crate) struct Registry {
    /// The `path:line:col type` suppression keys.
    keys: BTreeSet<String>,
}

impl Registry {
    /// Load the registry from the workspace root; empty if the file is absent.
    pub(crate) fn load(root: &Path) -> Self {
        let keys = std::fs::read_to_string(root.join(".claude/rules/no-bare-types-exemptions.txt"))
            .map(|text| {
                text.lines()
                    .map(str::trim)
                    .filter(|l| !l.is_empty() && !l.starts_with('#'))
                    .filter_map(entry_key)
                    .collect()
            })
            .unwrap_or_default();
        Self { keys }
    }

    /// Whether `key` is suppressed by a registered entry.
    pub(crate) fn contains(&self, key: &str) -> bool {
        self.keys.contains(key)
    }

    /// The registered keys not present in `live` — stale entries to remove.
    pub(crate) fn stale(&self, live: &BTreeSet<String>) -> Vec<String> {
        self.keys.difference(live).cloned().collect()
    }
}

/// The suppression key of a registry line — its first two whitespace tokens
/// (`<path:line:col>` and `<type>`) rejoined with a single space.
fn entry_key(line: &str) -> Option<String> {
    let mut tokens = line.split_whitespace();
    let loc = tokens.next()?;
    let ty = tokens.next()?;
    Some(format!("{loc} {ty}"))
}
