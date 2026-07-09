//! The SECTION-2 re-key machinery: pure pairing/rewrite helpers plus the opt-in
//! `NO_BARE_TYPES_REGEN=1` writer. The hermetic proofs of this logic live in the
//! sibling [`super::proofs`] module.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
};

use crate::{
    registry::{Entry, Registry, SECTION_TWO_HEADER, Section},
    tree::{production_rs, workspace_root},
    types::{ColumnNumber, RepoPath, TypeName, Violation},
    walk::scan_source,
};

/// The outcome of a SECTION-2 re-key: the rewritten entry lines (sorted by key),
/// the net-new live violations that were REFUSED (printed, never added), and the
/// stale entries that had no live pairing (dropped from the rewrite).
pub(super) struct RegenResult {
    /// Rewritten SECTION-2 entry lines, canonically sorted by their new key.
    pub(super) lines:    Vec<String>,
    /// Keys of live violations with no SECTION-2 entry to pair to — refused as
    /// net-new (user-approval-only), printed and left OUT of the rewrite.
    pub(super) excluded: Vec<String>,
    /// Keys of SECTION-2 entries with no live pairing — dropped as stale.
    pub(super) dropped:  Vec<String>,
}

/// The `(file, column, type)` a pairing groups on — line is the drift-prone
/// coordinate paired positionally WITHIN a group.
type GroupKey = (RepoPath, ColumnNumber, TypeName);

/// Re-key SECTION-2 `old` entries against the `live` scan. Within each
/// `(file, column, type)` group, entries and violations are paired positionally
/// by ascending line; a paired entry keeps its why-text and takes the live line.
/// Live violations already covered by a SECTION-1 key are excluded from the pool
/// (SECTION 1 is never folded in); any remaining unpaired live violation is
/// REFUSED (net-new) and any unpaired entry is dropped (stale).
pub(super) fn rekey_section_2(
    old: &[&Entry],
    live: &[Violation],
    section_one_keys: &BTreeSet<String>,
) -> RegenResult {
    let mut old_groups: BTreeMap<GroupKey, Vec<&Entry>> = BTreeMap::new();
    for entry in old {
        old_groups
            .entry((entry.path.clone(), entry.column, entry.type_name.clone()))
            .or_default()
            .push(entry);
    }
    for group in old_groups.values_mut() {
        group.sort_by_key(|entry| entry.line);
    }

    let mut live_groups: BTreeMap<GroupKey, Vec<&Violation>> = BTreeMap::new();
    for violation in live {
        if section_one_keys.contains(&violation.key()) {
            continue; // never fold a SECTION-1 false positive into SECTION 2
        }
        live_groups
            .entry((
                violation.path.clone(),
                violation.column,
                violation.type_name.clone(),
            ))
            .or_default()
            .push(violation);
    }
    for group in live_groups.values_mut() {
        group.sort_by_key(|violation| violation.line);
    }

    let mut lines = Vec::new();
    let mut excluded = Vec::new();
    let mut dropped = Vec::new();
    let mut group_keys: BTreeSet<GroupKey> = old_groups.keys().cloned().collect();
    group_keys.extend(live_groups.keys().cloned());
    let empty_entries: Vec<&Entry> = Vec::new();
    let empty_violations: Vec<&Violation> = Vec::new();
    for group_key in &group_keys {
        let entry_group = old_groups.get(group_key).unwrap_or(&empty_entries);
        let violation_group = live_groups.get(group_key).unwrap_or(&empty_violations);
        let paired = entry_group.len().min(violation_group.len());
        for (entry, violation) in entry_group.iter().zip(violation_group.iter()) {
            let key = violation.key();
            lines.push((key.clone(), format!("{key} {}", entry.why)));
        }
        for entry in &entry_group[paired..] {
            dropped.push(entry.key.clone());
        }
        for violation in &violation_group[paired..] {
            excluded.push(violation.key());
        }
    }
    lines.sort();
    RegenResult {
        lines: lines.into_iter().map(|(_, line)| line).collect(),
        excluded,
        dropped,
    }
}

/// Rewrite registry `original` with `section_2_lines`, copying everything up to
/// and including the SECTION-2 header block's closing `# ====` rule VERBATIM (so
/// SECTION 1 and every comment survive untouched) and replacing the SECTION-2
/// body. The header is matched on the EXACT [`SECTION_TWO_HEADER`] prefix — never
/// a prose comment that merely mentions "SECTION 2" (that prefix collision
/// deleted SECTION 1 before the fix). `None` if the header/closing markers are
/// absent.
pub(super) fn rewrite_text(original: &str, section_2_lines: &[String]) -> Option<String> {
    let lines: Vec<&str> = original.lines().collect();
    let header = lines
        .iter()
        .position(|l| l.trim_start().starts_with(SECTION_TWO_HEADER))?;
    let close_offset = lines[header..]
        .iter()
        .position(|l| l.trim_start().starts_with("# ===="))?;
    let close = header + close_offset;
    let mut out = String::new();
    for line in &lines[..=close] {
        out.push_str(line);
        out.push('\n');
    }
    for line in section_2_lines {
        out.push_str(line);
        out.push('\n');
    }
    Some(out)
}

/// Collect every live violation from the tracked production tree (both
/// suppressed and not — the re-key needs the full live set).
fn scan_live(root: &std::path::Path) -> Vec<Violation> {
    let mut live = Vec::new();
    for path in production_rs(root) {
        let Ok(src) = fs::read_to_string(root.join(&*path)) else {
            continue;
        };
        if let Ok(found) = scan_source(&path, &src) {
            live.extend(found);
        }
    }
    live
}

/// GTW-704 C5 — the opt-in re-key writer. Under `NO_BARE_TYPES_REGEN=1` it runs
/// the real scan, re-keys SECTION 2 to the live lines, prints every refused
/// net-new / dropped-stale key, and rewrites the registry file (SECTION 1
/// untouched). Without the env flag it early-returns, so a normal suite run
/// never mutates the repo.
#[test]
fn regenerate_section_2_keys() {
    if std::env::var_os("NO_BARE_TYPES_REGEN").is_none() {
        return; // opt-in maintenance mode only
    }
    let root = workspace_root();
    let registry = Registry::load(&root);
    let old = registry.entries_in(Section::Two);
    let section_one_keys = registry.section_one_keys();
    let live = scan_live(&root);

    let result = rekey_section_2(&old, &live, &section_one_keys);
    for key in &result.excluded {
        eprintln!("regen: REFUSED net-new (user-approval-only, left OUT): {key}");
    }
    for key in &result.dropped {
        eprintln!("regen: dropped stale (no live pairing): {key}");
    }

    let path = root.join(".claude/rules/no-bare-types-exemptions.txt");
    let Ok(original) = fs::read_to_string(&path) else {
        unreachable!("registry file must be readable to regenerate");
    };
    let Some(rewritten) = rewrite_text(&original, &result.lines) else {
        unreachable!("registry must carry the SECTION-2 header markers");
    };
    let Ok(()) = fs::write(&path, rewritten) else {
        unreachable!("registry file must be writable to regenerate");
    };
    eprintln!(
        "regen: wrote {} SECTION-2 entries ({} refused, {} dropped)",
        result.lines.len(),
        result.excluded.len(),
        result.dropped.len()
    );
}
