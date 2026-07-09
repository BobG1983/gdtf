//! The exemption registry — `.claude/rules/no-bare-types-exemptions.txt`,
//! mirroring `module-layout-exemptions.txt`. One entry per non-comment line:
//! `<path:line:col> <type> — <why> (approved/tracked: GTW-N, <date>)`. The
//! first two whitespace tokens (`<path:line:col>` and `<type>`) form the key
//! matched against [`crate::types::Violation::key`].
//!
//! The file is split into two labelled sections (`# SECTION 1` / `# SECTION 2`
//! header comments): SECTION 1 holds documented FALSE POSITIVES (checker
//! limitations, permanent, user-approval-only additions) and SECTION 2 holds
//! the GTW-599 BASELINE of known genuine violations pending follow-up. The
//! conformance test suppresses BOTH and FAILS on any STALE entry (a key that no
//! longer corresponds to a live violation). GTW-704 adds a SHRINK-ONLY guard on
//! SECTION 2 (see [`crate::ceiling`]) and a re-keying helper (see
//! [`crate::regen`]), both of which consume the per-entry structure parsed here.

use std::{collections::BTreeSet, path::Path};

use crate::types::{ColumnNumber, LineNumber, RepoPath, TypeName};

/// The EXACT prefix of the SECTION 1 header comment line (em-dash form). Matched
/// against a trimmed line so a prose comment that merely mentions "SECTION 1"
/// cannot be mistaken for the header — only the real `# SECTION 1 — …` rule flips
/// the parser/rewriter into SECTION 1.
pub(crate) const SECTION_ONE_HEADER: &str = "# SECTION 1 —";

/// The EXACT prefix of the SECTION 2 header comment line (em-dash form). Matched
/// against a trimmed line so a prose comment that merely mentions "SECTION 2"
/// (e.g. the registry's shrink-only note) cannot be mistaken for the header —
/// only the real `# SECTION 2 — …` rule flips the parser/rewriter into SECTION 2.
pub(crate) const SECTION_TWO_HEADER: &str = "# SECTION 2 —";

/// Which labelled section of the registry an entry sits in.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Section {
    /// SECTION 1 — documented false positives (permanent, user-approval-only).
    One,
    /// SECTION 2 — the GTW-599 baseline of known violations (shrink-only).
    Two,
}

/// A parsed registry entry: its section, its stable suppression key, the
/// re-keyable source coordinate, and the verbatim why-text that follows the key
/// (the `— <why>` remainder, preserved byte-for-byte for the regen helper).
pub(crate) struct Entry {
    /// The section this entry was listed under.
    pub(crate) section:   Section,
    /// The `path:line:col type` suppression key.
    pub(crate) key:       String,
    /// Repo-relative source path (the `path` of `path:line:col`).
    pub(crate) path:      RepoPath,
    /// 1-based line of the offending type — the drift-prone coordinate.
    pub(crate) line:      LineNumber,
    /// 1-based column of the offending type.
    pub(crate) column:    ColumnNumber,
    /// The offending bare type name (the `<type>` token).
    pub(crate) type_name: TypeName,
    /// The verbatim remainder after the key (`— <why> …`), preserved on regen.
    pub(crate) why:       String,
}

/// The parsed registry — the suppression keys (both sections, for the
/// conformance stale/contains checks) plus the per-entry structure (for the
/// GTW-704 ceiling guard and regen helper).
pub(crate) struct Registry {
    /// The `path:line:col type` suppression keys across BOTH sections.
    keys:    BTreeSet<String>,
    /// Every parsed entry, in file order, tagged with its section.
    entries: Vec<Entry>,
}

impl Registry {
    /// Load the registry from the workspace root; empty if the file is absent.
    pub(crate) fn load(root: &Path) -> Self {
        std::fs::read_to_string(root.join(".claude/rules/no-bare-types-exemptions.txt"))
            .map_or_else(|_| Self::parse(""), |text| Self::parse(&text))
    }

    /// Parse registry `text` into keys + per-entry structure. Section membership
    /// is tracked by the EXACT section-header comments [`SECTION_ONE_HEADER`] /
    /// [`SECTION_TWO_HEADER`] (the em-dash form, so a prose comment that merely
    /// mentions "SECTION 2" cannot flip the parser mid-file); entries before
    /// either header are ignored (there are none in practice).
    pub(crate) fn parse(text: &str) -> Self {
        let mut section = None;
        let mut entries = Vec::new();
        for raw in text.lines() {
            let line = raw.trim();
            if line.starts_with(SECTION_ONE_HEADER) {
                section = Some(Section::One);
                continue;
            }
            if line.starts_with(SECTION_TWO_HEADER) {
                section = Some(Section::Two);
                continue;
            }
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some(entry) = parse_entry(line, section) {
                entries.push(entry);
            }
        }
        let keys = entries.iter().map(|e| e.key.clone()).collect();
        Self { keys, entries }
    }

    /// Whether `key` is suppressed by a registered entry (either section).
    pub(crate) fn contains(&self, key: &str) -> bool {
        self.keys.contains(key)
    }

    /// The registered keys not present in `live` — stale entries to remove.
    pub(crate) fn stale(&self, live: &BTreeSet<String>) -> Vec<String> {
        self.keys.difference(live).cloned().collect()
    }

    /// Every entry listed under `section`, in file order.
    pub(crate) fn entries_in(&self, section: Section) -> Vec<&Entry> {
        self.entries
            .iter()
            .filter(|e| e.section == section)
            .collect()
    }

    /// The number of entries listed under `section`.
    pub(crate) fn count_in(&self, section: Section) -> usize {
        self.entries.iter().filter(|e| e.section == section).count()
    }

    /// The suppression keys of every SECTION-1 entry — the false-positive keys
    /// the regen helper must never fold into SECTION 2.
    pub(crate) fn section_one_keys(&self) -> BTreeSet<String> {
        self.entries
            .iter()
            .filter(|e| e.section == Section::One)
            .map(|e| e.key.clone())
            .collect()
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

/// Parse one non-comment registry line into a structured [`Entry`], preserving
/// the verbatim why-text. Returns `None` if the line lacks a `path:line:col`
/// location with numeric line/column or a `<type>` token.
fn parse_entry(line: &str, section: Option<Section>) -> Option<Entry> {
    let section = section?;
    let loc_end = line.find(char::is_whitespace)?;
    let loc = &line[..loc_end];
    let tail = line[loc_end..].trim_start();
    let ty_end = tail.find(char::is_whitespace).unwrap_or(tail.len());
    let ty = &tail[..ty_end];
    if ty.is_empty() {
        return None;
    }
    let why = tail[ty_end..].trim_start().to_owned();
    let (path, line_no, column) = parse_loc(loc)?;
    Some(Entry {
        section,
        key: entry_key(line)?,
        path,
        line: line_no,
        column,
        type_name: TypeName::new(ty),
        why,
    })
}

/// Split a `path:line:col` location into its parts. Line and column are the last
/// two colon-separated numeric fields; everything before them is the path (so a
/// path containing no colon is handled, matching the tracked-tree paths).
fn parse_loc(loc: &str) -> Option<(RepoPath, LineNumber, ColumnNumber)> {
    let mut parts = loc.rsplitn(3, ':');
    let col = parts.next()?.parse::<usize>().ok()?;
    let line = parts.next()?.parse::<usize>().ok()?;
    let path = parts.next()?;
    Some((
        RepoPath::new(path),
        LineNumber::new(line),
        ColumnNumber::new(col),
    ))
}
