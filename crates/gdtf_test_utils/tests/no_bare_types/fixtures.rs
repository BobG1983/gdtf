//! Hermetic checker-behavior tests (AC2 and the round-1 fix discriminators).
//! Each runs [`scan_source`] over an in-test source STRING the test owns — no
//! repo file is touched — and asserts exactly which bare types are (or are not)
//! flagged. These pin the structural allowlist to EXACTLY rules 4/5 + Q4: a
//! test here would FAIL if a carve-out were re-loosened past the rule.

use crate::{
    types::{LineNumber, PositionKind, RepoPath},
    walk::scan_source,
};

/// Parse a fixture string and return its violations (the fixture must parse).
/// Shared with [`crate::conventions_fixtures`] (the GTW-722 carve-out fixtures).
pub(crate) fn scan(src: &str) -> Vec<crate::types::Violation> {
    let path = RepoPath::new("FIXTURE.rs");
    let Ok(found) = scan_source(&path, src) else {
        unreachable!("fixture must parse");
    };
    found
}

/// AC2 — a hermetic proof the checker WORKS: a bare `u32` struct field is
/// flagged at the expected line with the expected type, position, and message.
#[test]
fn flags_synthetic_bare_field() {
    let found = scan(
        "\
/// A synthetic domain type with a bare field.
pub struct Ganger {
    pub hp: u32,
}
",
    );
    assert_eq!(found.len(), 1, "exactly the one bare field: {found:?}");
    let Some(violation) = found.first() else {
        unreachable!("one violation was asserted");
    };
    assert_eq!(&*violation.type_name, "u32");
    assert_eq!(violation.kind, PositionKind::StructField);
    assert_eq!(violation.line, LineNumber::new(3), "the `hp: u32` line");
    assert!(
        violation
            .render()
            .contains("bare `u32` used as a domain value"),
        "clippy-style message: {}",
        violation.render()
    );
}

/// AC2 counterpart — a newtype-clean fixture yields ZERO violations: the `u32`
/// inner of `Hp`, its constructor param (matches the inner), and a `Hp`-typed
/// field are all exempt (rules 4/5).
#[test]
fn newtype_fixture_is_clean() {
    let found = scan(
        "\
/// Hit points.
pub struct Hp(u32);

impl Hp {
    /// Construct from a raw count.
    pub fn new(value: u32) -> Self {
        Self(value)
    }
}

/// A ganger holding only wrapped values.
pub struct Ganger {
    hp: Hp,
}
",
    );
    assert!(found.is_empty(), "newtype-clean fixture: {found:?}");
}

/// Problem #2 — a single-field tuple ENUM VARIANT is a variant, not a newtype,
/// so its bare field is flagged (the newtype carve-out is struct-only).
#[test]
fn enum_single_tuple_variant_is_flagged() {
    let found = scan(
        "\
/// A synthetic error enum.
pub enum RonSaveError {
    Serialize(String),
}
",
    );
    assert_eq!(found.len(), 1, "the variant's bare field: {found:?}");
    let Some(violation) = found.first() else {
        unreachable!("one violation was asserted");
    };
    assert_eq!(&*violation.type_name, "String");
    assert_eq!(violation.kind, PositionKind::EnumField);
}

/// Problem #1 — a `#[derive(Deserialize)]` wire shape is NOT structurally
/// carved out; its bare field is flagged (newtypes serialize fine).
#[test]
fn serde_derive_field_is_flagged() {
    let found = scan(
        "\
use serde::Deserialize;

/// A wire shape read from disk.
#[derive(Deserialize)]
pub struct Payload {
    pub count: u32,
}
",
    );
    assert_eq!(found.len(), 1, "the serde field: {found:?}");
    let Some(violation) = found.first() else {
        unreachable!("one violation was asserted");
    };
    assert_eq!(&*violation.type_name, "u32");
    assert_eq!(violation.kind, PositionKind::StructField);
}

/// Problem #4b — a bare `bool` RETURN is flagged (no blanket predicate
/// carve-out); problem #4c — a bare `usize` PARAM is flagged (no blanket
/// collection-index carve-out).
#[test]
fn bool_return_and_usize_param_are_flagged() {
    let found = scan(
        "\
/// A free function trafficking in bare plumbing.
pub fn probe(index: usize) -> bool {
    index == 0
}
",
    );
    let types: Vec<&str> = found.iter().map(|v| &**v.type_name).collect();
    assert!(types.contains(&"usize"), "usize param flagged: {found:?}");
    assert!(types.contains(&"bool"), "bool return flagged: {found:?}");
    assert_eq!(found.len(), 2, "exactly the two: {found:?}");
}

/// Problem #3 — inside a newtype's OWN inherent `impl`, only a bare
/// param/return matching the inner type is exempt; a DIFFERENT bare type is
/// flagged (the constructor `u32` is exempt, the `String` label return is not).
#[test]
fn newtype_impl_exempts_only_the_inner_type() {
    let found = scan(
        "\
/// Hit points.
pub struct Hp(u32);

impl Hp {
    /// Matches the inner — exempt.
    pub fn new(value: u32) -> Self {
        Self(value)
    }

    /// Does NOT match the inner — flagged.
    pub fn label(&self) -> String {
        String::new()
    }
}
",
    );
    assert_eq!(found.len(), 1, "only the non-inner return: {found:?}");
    let Some(violation) = found.first() else {
        unreachable!("one violation was asserted");
    };
    assert_eq!(&*violation.type_name, "String");
    assert_eq!(violation.kind, PositionKind::FnReturn);
}

/// Rule 4 (kept) — a trait `impl` signature is exempt; problem #4a — the trait
/// DEFINITION's signature for the same method IS flagged.
#[test]
fn trait_def_flagged_but_trait_impl_exempt() {
    let def = scan(
        "\
/// A trait whose signature is a domain-modeling choice.
pub trait Named {
    /// Definition signature — flagged.
    fn set_count(&mut self, count: u32);
}
",
    );
    assert_eq!(def.len(), 1, "trait-def sig flagged: {def:?}");
    let Some(violation) = def.first() else {
        unreachable!("one violation was asserted");
    };
    assert_eq!(&*violation.type_name, "u32");
    assert_eq!(violation.kind, PositionKind::FnParam);

    let imp = scan(
        "\
/// A trait.
pub trait Named {
    /// Definition signature.
    fn set_count(&mut self, count: u32);
}

/// A holder.
pub struct Holder;

impl Named for Holder {
    fn set_count(&mut self, count: u32) {
        let _ = count;
    }
}
",
    );
    // Only the trait-definition sig is flagged; the impl sig is exempt (rule 4).
    assert_eq!(imp.len(), 1, "impl sig exempt, def sig flagged: {imp:?}");
    let Some(violation) = imp.first() else {
        unreachable!("one violation was asserted");
    };
    assert_eq!(violation.line, LineNumber::new(4), "the trait-DEF line");
}
