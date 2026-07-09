//! Hermetic + shipped-file regression proofs for the SECTION-2 re-key machinery
//! (see [`super::machinery`]). No repo file is mutated — the writer's file I/O is
//! exercised only in-memory (`rewrite_text`) or against synthetic input.

use std::collections::BTreeSet;

use super::machinery::{rekey_section_2, rewrite_text};
use crate::{
    registry::{Registry, SECTION_ONE_HEADER, SECTION_TWO_HEADER, Section},
    tree::workspace_root,
    types::{ColumnNumber, LineNumber, PositionKind, RepoPath, TypeName, Violation},
};

/// GTW-704 C5 (hermetic) — the pairing preserves each entry's why-text under
/// line drift, and an unpairable live violation is REFUSED (excluded and named),
/// never turned into a net-new entry. No repo file is touched.
#[test]
fn rekey_preserves_why_and_refuses_net_new() {
    let registry = Registry::parse(
        "\
# ============================================================================
# SECTION 2 — GTW-599 BASELINE.
# ============================================================================
crates/x/src/a.rs:10:5 u32 — FIELD-NUMERIC; original why-text A
crates/x/src/a.rs:20:5 u32 — FIELD-NUMERIC; original why-text B
crates/x/src/gone.rs:9:1 bool — SIG-BOOL; stale, no live pairing
",
    );
    let old = registry.entries_in(Section::Two);

    // Same (file,col,type) group as the two `a.rs` entries, but lines DRIFTED
    // down by two; plus one net-new violation in a group with no entry.
    let live = vec![
        violation("crates/x/src/a.rs", 12, 5, "u32"),
        violation("crates/x/src/a.rs", 22, 5, "u32"),
        violation("crates/x/src/b.rs", 7, 3, "bool"),
    ];

    let result = rekey_section_2(&old, &live, &BTreeSet::new());

    assert_eq!(
        result.lines.len(),
        2,
        "two entries re-keyed: {:?}",
        result.lines
    );
    assert!(
        result
            .lines
            .iter()
            .any(|l| l == "crates/x/src/a.rs:12:5 u32 — FIELD-NUMERIC; original why-text A"),
        "line drift 10->12 with why preserved: {:?}",
        result.lines
    );
    assert!(
        result
            .lines
            .iter()
            .any(|l| l == "crates/x/src/a.rs:22:5 u32 — FIELD-NUMERIC; original why-text B"),
        "line drift 20->22 with why preserved: {:?}",
        result.lines
    );
    assert_eq!(
        result.excluded,
        vec!["crates/x/src/b.rs:7:3 bool".to_owned()]
    );
    assert_eq!(
        result.dropped,
        vec!["crates/x/src/gone.rs:9:1 bool".to_owned()],
        "the stale entry is dropped, not re-keyed"
    );
}

/// GTW-704 C5 (hermetic) — the re-key REFUSES to fold a SECTION-1 false positive
/// into SECTION 2: a live violation whose key matches a SECTION-1 entry is
/// excluded from the pairing pool entirely and is neither re-keyed nor refused.
#[test]
fn rekey_never_folds_section_one() {
    let registry = Registry::parse(
        "\
# SECTION 2 — GTW-599 BASELINE.
crates/x/src/a.rs:10:5 u32 — FIELD-NUMERIC; keep
",
    );
    let old = registry.entries_in(Section::Two);
    let live = vec![
        violation("crates/x/src/a.rs", 10, 5, "u32"),
        violation("crates/fog/src/m.rs", 1, 1, "Vec2"),
    ];
    let mut section_one = BTreeSet::new();
    section_one.insert("crates/fog/src/m.rs:1:1 Vec2".to_owned());
    // The fog violation's key is a SECTION-1 key, so it is filtered before
    // grouping; only the `a.rs` pair survives and nothing is refused.
    let result = rekey_section_2(&old, &live, &section_one);
    assert_eq!(
        result.lines.len(),
        1,
        "only the a.rs entry: {:?}",
        result.lines
    );
    assert!(
        result.excluded.is_empty(),
        "SECTION-1 violation not refused as net-new: {:?}",
        result.excluded
    );
}

/// GTW-704 C5 (hermetic) — `rewrite_text` copies everything through the
/// SECTION-2 header VERBATIM (SECTION 1 untouched) and swaps in the new body.
/// The fixture deliberately includes a top-of-file prose line that MENTIONS
/// "# SECTION 2" (mirroring the shipped registry's shrink-only note) to prove the
/// exact-header anchor is not fooled by the prefix collision.
#[test]
fn rewrite_preserves_section_one() {
    let original = "\
# header
# When an edit drifts a baselined line, re-key
# SECTION 2 mechanically instead of by hand (prose — NOT the header).
# ============================================================================
# SECTION 1 — DOCUMENTED FALSE POSITIVES (permanent).
# ============================================================================
crates/fog/src/m.rs:1:1 Vec2 — FALSE POSITIVE (approved: GTW-X)
# ============================================================================
# SECTION 2 — GTW-599 BASELINE.
# ============================================================================
crates/old/src/a.rs:99:1 u32 — stale line to be replaced
";
    let body = vec!["crates/new/src/a.rs:5:1 u32 — FIELD-NUMERIC; kept".to_owned()];
    let Some(out) = rewrite_text(original, &body) else {
        unreachable!("headers present");
    };
    assert!(
        out.contains("crates/fog/src/m.rs:1:1 Vec2 — FALSE POSITIVE (approved: GTW-X)"),
        "SECTION 1 preserved verbatim:\n{out}"
    );
    assert!(
        out.contains("crates/new/src/a.rs:5:1 u32"),
        "new body written:\n{out}"
    );
    assert!(
        !out.contains("crates/old/src/a.rs:99:1"),
        "old SECTION-2 body replaced:\n{out}"
    );
}

/// GTW-704 C5 (shipped-file fidelity) — rewrite the ACTUAL tracked registry text
/// in-memory and assert every SECTION-1 line (header, comments, and all
/// false-positive entries) survives byte-for-byte. This is the regression that
/// the synthetic [`rewrite_preserves_section_one`] fixture could NOT catch: the
/// shipped registry's top-of-file note contains a `# SECTION 2 …` prose line, so
/// a prefix-match anchor resolved the header ABOVE SECTION 1 and a real regen run
/// deleted the whole SECTION-1 block. No repo file is touched (pure in-memory
/// rewrite).
#[test]
fn rewrite_preserves_shipped_section_one() {
    let root = workspace_root();
    let path = root.join(".claude/rules/no-bare-types-exemptions.txt");
    let Ok(original) = std::fs::read_to_string(&path) else {
        unreachable!("shipped registry must be readable");
    };

    // The verbatim SECTION-1 block: from its header up to (not into) SECTION 2.
    let section_one_block: Vec<&str> = original
        .lines()
        .skip_while(|l| !l.trim_start().starts_with(SECTION_ONE_HEADER))
        .take_while(|l| !l.trim_start().starts_with(SECTION_TWO_HEADER))
        .collect();
    assert!(
        section_one_block
            .first()
            .is_some_and(|l| l.trim_start().starts_with(SECTION_ONE_HEADER)),
        "shipped registry must have a SECTION-1 header to protect"
    );
    let parsed = Registry::parse(&original);
    let section_one_entries = parsed.entries_in(Section::One);
    assert!(
        !section_one_entries.is_empty(),
        "shipped registry must carry SECTION-1 false-positive entries"
    );

    let body = vec!["crates/x/src/synthetic.rs:1:1 u32 — FIELD-NUMERIC; probe".to_owned()];
    let Some(out) = rewrite_text(&original, &body) else {
        unreachable!("shipped registry must carry the SECTION-2 header markers");
    };

    // Every SECTION-1 line (header + comments + each entry) survives verbatim.
    for line in &section_one_block {
        assert!(
            out.contains(line),
            "SECTION-1 line dropped by rewrite (prefix-collision regression):\n  {line}"
        );
    }
    // The new SECTION-2 body was written and the old SECTION-2 entries replaced.
    assert!(
        out.contains("crates/x/src/synthetic.rs:1:1 u32"),
        "new SECTION-2 body present:\n{out}"
    );
}

/// Build a synthetic [`Violation`] at a coordinate for the hermetic tests.
fn violation(path: &str, line: usize, column: usize, ty: &str) -> Violation {
    Violation {
        path:      RepoPath::new(path),
        line:      LineNumber::new(line),
        column:    ColumnNumber::new(column),
        type_name: TypeName::new(ty),
        kind:      PositionKind::StructField,
    }
}
