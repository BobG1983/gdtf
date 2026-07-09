//! GTW-704 C5 — the SECTION-2 REGENERATE helper for line-drift re-keying.
//!
//! Every land that edits source ABOVE a baselined line shifts that entry's
//! `path:line:col` key, so the entry goes stale and a hand-scripted re-key is
//! needed (27 keys at the GTW-599 land alone). This helper does that
//! mechanically: run the real scan, pair each existing SECTION-2 entry to the
//! live violation at its `(file, column, type)` — ordered by line so drift is
//! absorbed — and rewrite the key to the live line while PRESERVING the entry's
//! verbatim why-text and class tag.
//!
//! Two hard invariants keep it safe: it REFUSES to produce net-new entries (a
//! live violation with no existing SECTION-2 entry to pair to is printed and
//! left OUT — adding entries stays user-approval-only), and it REFUSES to touch
//! SECTION 1 (false-positive entries are excluded from the live pairing pool and
//! the file's SECTION-1 text is copied through byte-for-byte).
//!
//! It is opt-in: the writer only runs under `NO_BARE_TYPES_REGEN=1`, e.g.
//! `NO_BARE_TYPES_REGEN=1 cargo test -p gdtf_test_utils --test no_bare_types
//! regenerate_section_2_keys -- --nocapture`. A normal suite run early-returns.
//!
//! Split by concern: [`machinery`] holds the re-key logic + the env-gated writer;
//! [`proofs`] holds the hermetic + shipped-file regression tests.

mod machinery;
mod proofs;
