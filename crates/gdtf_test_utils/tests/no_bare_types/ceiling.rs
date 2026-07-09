//! GTW-704 — the SHRINK-ONLY guard on SECTION 2 of the exemption registry
//! (user ruling 2026-07-09: the registry is shrink-only). SECTION 2 (the
//! GTW-599 baseline of known pre-existing violations) may only ever shrink; it
//! must never grow. This module pins the current SECTION-2 entry count as a
//! ceiling and fails the suite if a run finds more entries than the pin — so a
//! new bare type can be fixed (wrap it) or, with user approval, the pin raised,
//! but it can never be silently baselined away.
//!
//! SECTION 1 (documented false positives) is EXEMPT from the ceiling — a genuine
//! new false positive is a legitimate addition — but its additions remain
//! user-approval-only per the registry header, so a growth past its pinned
//! baseline prints a LOUD, non-failing reminder to route the addition through
//! the approval the header requires.

use std::io::Write;

use crate::{
    registry::{Registry, Section},
    tree::workspace_root,
};

/// The pinned ceiling on SECTION-2 (baseline) entries. Measured live at the
/// GTW-704 land: 860 GTW-599 baseline entries + 5 GTW-587-introduced entries
/// (user-approved 2026-07-09) = 865. SHRINK-ONLY: any commit may LOWER this
/// (each burn-down wave's land does); it is raised only with user approval.
pub(crate) const SECTION_2_CEILING: usize = 865;

/// The pinned baseline count of SECTION-1 (documented false positive) entries.
/// Growth past this is legal but user-approval-only, so it triggers the loud
/// non-failing reminder in [`section_1_growth_reminder`], not a failure.
pub(crate) const SECTION_1_BASELINE: usize = 3;

/// The ceiling diagnostic for a SECTION-2 count, or `None` when the count is at
/// or below the pin (shrinking / holding stays green). Pure over the count so
/// the hermetic fixture can drive it with a synthetic registry and ceiling.
pub(crate) fn section_2_ceiling_diagnostic(count: usize, ceiling: usize) -> Option<String> {
    if count <= ceiling {
        return None;
    }
    let over = count - ceiling;
    Some(format!(
        "no-bare-types: SECTION 2 of the exemption registry GREW — {count} entries \
         against the pinned ceiling of {ceiling} ({over} over budget).\n  The registry \
         is SHRINK-ONLY (GTW-704): no NEW SECTION-2 baseline entries may be added. Wrap \
         the offending value(s) in a named newtype instead of baselining, or — with \
         explicit user approval — raise SECTION_2_CEILING.\n  To name the added \
         entries, inspect the registry's last change (works whether the growth is \
         staged, unstaged, OR already committed):\n    git diff -- \
         .claude/rules/no-bare-types-exemptions.txt        # staged/unstaged growth\n    \
         git log -p -1 -- .claude/rules/no-bare-types-exemptions.txt  # committed growth\n  \
         (each ADDED line under `# SECTION 2 —` is a new bare-type use that must be \
         fixed, not baselined.)"
    ))
}

/// Emit `message` to the process stderr FD directly, bypassing libtest's
/// print-macro capture. C2 requires the SECTION-1 growth reminder to be VISIBLE
/// even though its test PASSES; `eprintln!` inside a passing test is swallowed by
/// libtest's `set_output_capture`, so the reminder is written straight to the fd
/// (best-effort — a failed write must not fail the non-failing reminder).
fn emit_to_stderr(message: &str) {
    let mut stderr = std::io::stderr().lock();
    let _written = stderr.write_all(message.as_bytes());
    let _newline = stderr.write_all(b"\n");
}

/// The loud non-failing reminder for SECTION-1 growth, or `None` when it holds
/// at/below the baseline. Pure over the counts for the hermetic fixture.
pub(crate) fn section_1_growth_reminder(count: usize, baseline: usize) -> Option<String> {
    if count <= baseline {
        return None;
    }
    let added = count - baseline;
    Some(format!(
        "no-bare-types: REMINDER — SECTION 1 (documented false positives) grew from the \
         pinned baseline of {baseline} to {count} (+{added}). This is not a failure, but \
         SECTION-1 additions are USER-APPROVAL-ONLY per the registry header — confirm the \
         new false-positive entr(y/ies) are approved and update SECTION_1_BASELINE."
    ))
}

/// GTW-704 C1/C2 — SECTION 2 is shrink-only. Load the live registry, print the
/// SECTION-1 growth reminder if it grew (non-failing, C2), and FAIL if SECTION 2
/// exceeds the pinned ceiling (C1). Does NOT re-scan the tree: this guards the
/// registry file's own SECTION-2 count, independent of the tree-wide walk in
/// [`crate::conformance`].
#[test]
fn section_2_registry_is_shrink_only() {
    let root = workspace_root();
    let registry = Registry::load(&root);

    let section_1 = registry.count_in(Section::One);
    if let Some(reminder) = section_1_growth_reminder(section_1, SECTION_1_BASELINE) {
        // Direct-to-fd: this test PASSES on SECTION-1 growth, and libtest would
        // otherwise capture (swallow) the reminder from a passing test.
        emit_to_stderr(&reminder);
    }

    let section_2 = registry.count_in(Section::Two);
    let diagnostic = section_2_ceiling_diagnostic(section_2, SECTION_2_CEILING);
    if let Some(message) = &diagnostic {
        eprintln!("{message}");
    }
    assert!(
        diagnostic.is_none(),
        "SECTION 2 exceeded its shrink-only ceiling — see the diagnostic above \
         ({section_2} entries, ceiling {SECTION_2_CEILING})"
    );
}

/// GTW-704 C3 — a hermetic proof the ceiling guard fails on a SYNTHETIC grown
/// registry (in-test string, no repo file touched — the GTW-599 AC2 pattern).
/// Parses a 3-entry SECTION-2 fixture and asserts the guard fires when the pin
/// is 2 (grown), stays green when the pin is 3 (held), and stays green when the
/// pin is 4 (shrunk relative to budget).
#[test]
fn ceiling_guard_fails_on_grown_registry() {
    let text = "\
# header comment
# ============================================================================
# SECTION 1 — DOCUMENTED FALSE POSITIVES (permanent).
# ============================================================================
crates/foo/src/a.rs:1:1 Vec2 — FALSE POSITIVE: ShaderType uniform (approved: GTW-X)
# ============================================================================
# SECTION 2 — GTW-599 BASELINE.
# ============================================================================
crates/foo/src/b.rs:10:5 u32 — FIELD-NUMERIC; baseline
crates/foo/src/c.rs:20:9 String — SIG-STRING; baseline
crates/foo/src/d.rs:30:13 bool — SIG-BOOL; baseline
";
    let registry = Registry::parse(text);
    assert_eq!(registry.count_in(Section::One), 1, "one SECTION-1 entry");
    assert_eq!(
        registry.count_in(Section::Two),
        3,
        "three SECTION-2 entries"
    );

    let count = registry.count_in(Section::Two);
    assert!(
        section_2_ceiling_diagnostic(count, 2).is_some(),
        "count 3 over a ceiling of 2 must fail"
    );
    let over = section_2_ceiling_diagnostic(count, 2).unwrap_or_default();
    assert!(
        over.contains("SHRINK-ONLY"),
        "diagnostic mentions the rule: {over}"
    );
    assert!(
        section_2_ceiling_diagnostic(count, 3).is_none(),
        "count 3 at a ceiling of 3 stays green"
    );
    assert!(
        section_2_ceiling_diagnostic(count, 4).is_none(),
        "count 3 below a ceiling of 4 stays green"
    );
}

/// GTW-704 C2 (hermetic) — the SECTION-1 growth reminder is non-failing: it
/// yields a message on growth and nothing when the count holds.
#[test]
fn section_1_growth_reminder_is_advisory() {
    assert!(
        section_1_growth_reminder(3, 3).is_none(),
        "holding at baseline is silent"
    );
    assert!(
        section_1_growth_reminder(2, 3).is_none(),
        "shrinking is silent"
    );
    let grown = section_1_growth_reminder(5, 3);
    assert!(grown.is_some(), "growth yields a reminder");
    let message = grown.unwrap_or_default();
    assert!(
        message.contains("USER-APPROVAL-ONLY"),
        "reminder points at the approval requirement: {message}"
    );
}

/// GTW-704 C3 (tempdir variant) — the disk `load` path parses sections from a
/// synthetic registry written into a throwaway temp directory (no repo file
/// touched); the ceiling guard then sees the grown SECTION-2 count.
#[test]
fn load_from_tempdir_sees_grown_section_2() {
    let dir =
        std::env::temp_dir().join(format!("gdtf_no_bare_types_ceiling_{}", std::process::id()));
    let rules = dir.join(".claude/rules");
    let file = rules.join("no-bare-types-exemptions.txt");
    let text = "\
# ============================================================================
# SECTION 1 — DOCUMENTED FALSE POSITIVES (permanent).
# ============================================================================
crates/foo/src/a.rs:1:1 Vec2 — FALSE POSITIVE (approved: GTW-X)
# ============================================================================
# SECTION 2 — GTW-599 BASELINE.
# ============================================================================
crates/foo/src/b.rs:10:5 u32 — FIELD-NUMERIC; baseline
crates/foo/src/c.rs:20:9 String — SIG-STRING; baseline
";
    let Ok(()) = std::fs::create_dir_all(&rules) else {
        unreachable!("temp dir must be creatable");
    };
    let Ok(()) = std::fs::write(&file, text) else {
        unreachable!("temp registry must be writable");
    };

    let registry = Registry::load(&dir);
    let section_2 = registry.count_in(Section::Two);
    let _cleanup = std::fs::remove_dir_all(&dir);

    assert_eq!(section_2, 2, "two SECTION-2 entries parsed from disk");
    assert!(
        section_2_ceiling_diagnostic(section_2, 1).is_some(),
        "count 2 over a ceiling of 1 fails"
    );
}
