//! Hermetic fixtures for the four GTW-722 convention carve-outs (see
//! [`crate::conventions`]). Each carve-out gets a POSITIVE fixture (the
//! convention is exempted) AND a discriminating NEGATIVE fixture (a near-miss
//! that fails the exact wording is STILL flagged). Like [`crate::fixtures`],
//! each scans an in-test source STRING the test owns — no repo file is touched.

use crate::{fixtures::scan, types::PositionKind};

/// Carve-out (a), POSITIVE — the std-container trio `len`/`is_empty`/`contains…`
/// on one inherent `impl` is fully exempt: the `usize` count, both `bool`
/// answers, and the reference-taking `contains` all clear.
#[test]
fn std_container_trio_is_exempt() {
    let found = scan(
        "\
/// A key type (not a bare flagged type).
pub struct Key;

/// A collection-surface newtype over a non-flagged inner.
pub struct Bag(Vec<Key>);

impl Bag {
    /// std-container len — paired with is_empty below.
    pub fn len(&self) -> usize {
        self.0.len()
    }
    /// std-container is_empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    /// std-container contains — takes a reference key.
    pub fn contains(&self, key: &Key) -> bool {
        let _ = key;
        false
    }
}
",
    );
    assert!(found.is_empty(), "the container trio is exempt: {found:?}");
}

/// Carve-out (a), NEGATIVE — a `len` with NO `is_empty` sibling and a `contains`
/// with NO reference parameter are BOTH still flagged (the near-misses the
/// pairing / reference-param gates exclude).
#[test]
fn len_without_is_empty_and_contains_without_ref_are_flagged() {
    let found = scan(
        "\
/// A key type.
pub struct Key;

/// A near-miss container: a lone len, and a contains with no reference param.
pub struct Counter(Vec<Key>);

impl Counter {
    /// A len with no is_empty sibling — NOT exempt.
    pub fn len(&self) -> usize {
        self.0.len()
    }
    /// A contains that takes no reference param — NOT exempt.
    pub fn contains(&self) -> bool {
        self.0.is_empty()
    }
}
",
    );
    let types: Vec<&str> = found.iter().map(|v| &**v.type_name).collect();
    assert_eq!(found.len(), 2, "both near-misses flagged: {found:?}");
    assert!(
        types.contains(&"usize"),
        "lone len usize flagged: {found:?}"
    );
    assert!(
        types.contains(&"bool"),
        "ref-less contains bool flagged: {found:?}"
    );
    assert!(
        found.iter().all(|v| v.kind == PositionKind::FnReturn),
        "both are return positions: {found:?}"
    );
}

/// Carve-out (a), NEGATIVE (own-receiver form, trio gate) — the trio quotes
/// `&self` exactly, so a `len(&mut self) -> usize` and an `is_empty(self) -> bool`
/// by value are BOTH still flagged: each fails the trio GATE on its OWN receiver
/// form (`&mut self` / by-value `self` are not `&self`). Pins the trio-gate
/// receiver-form tightening — a gate loosened to any receiver would wrongly
/// exempt the by-value `is_empty`, dropping the count to 1 (GTW-722 fix). The
/// sibling-form tightening is pinned separately by
/// [`unpaired_len_with_wrong_form_is_empty_sibling_is_flagged`].
#[test]
fn wrong_receiver_form_trio_is_flagged() {
    let found = scan(
        "\
/// A key type.
pub struct Key;

/// A near-miss container: wrong receiver forms on the trio methods.
pub struct Bag(Vec<Key>);

impl Bag {
    /// `&mut self`, not `&self` — fails the trio gate on its own receiver.
    pub fn len(&mut self) -> usize {
        self.0.len()
    }
    /// `self` by value, not `&self` — fails the trio gate on its own receiver.
    pub fn is_empty(self) -> bool {
        self.0.is_empty()
    }
}
",
    );
    let types: Vec<&str> = found.iter().map(|v| &**v.type_name).collect();
    assert_eq!(
        found.len(),
        2,
        "both wrong-receiver-form near-misses flagged: {found:?}"
    );
    assert!(
        types.contains(&"usize"),
        "`&mut self` len usize flagged: {found:?}"
    );
    assert!(
        types.contains(&"bool"),
        "by-value is_empty bool flagged: {found:?}"
    );
    assert!(
        found.iter().all(|v| v.kind == PositionKind::FnReturn),
        "both are return positions: {found:?}"
    );
}

/// Carve-out (a), NEGATIVE (sibling-receiver form) — a PROPER `len(&self) ->
/// usize` whose ONLY `is_empty` sibling is wrong-form (`is_empty(&mut self) ->
/// bool`, not `&self`) is still flagged: the `len` loses its qualifying
/// `is_empty(&self)` sibling (the `len_without_is_empty` pairing), and the
/// wrong-form `is_empty(&mut self)` itself fails the trio gate — BOTH flagged.
/// Pins the `declares_is_empty` sibling receiver-form tightening: the sibling
/// check requires the sibling be exactly `is_empty(&self)`, so loosening it to
/// any receiver would re-pair the `len` and drop the count to 1. (GTW-722 fix —
/// the r1 gate-only pin left this half untested; all 63 live class-(a) drops are
/// `&self`, so only this fixture discriminates the sibling-form revert.)
#[test]
fn unpaired_len_with_wrong_form_is_empty_sibling_is_flagged() {
    let found = scan(
        "\
/// A key type.
pub struct Key;

/// A near-miss container: a proper &self len, but a wrong-form is_empty sibling.
pub struct Bag(Vec<Key>);

impl Bag {
    /// Proper `&self` len — but its only is_empty sibling is `&mut self`, so it
    /// loses the qualifying pair and stays flagged.
    pub fn len(&self) -> usize {
        self.0.len()
    }
    /// `&mut self`, not `&self` — fails the trio gate, and does NOT qualify as
    /// the `len`'s `is_empty(&self)` sibling.
    pub fn is_empty(&mut self) -> bool {
        self.0.is_empty()
    }
}
",
    );
    let types: Vec<&str> = found.iter().map(|v| &**v.type_name).collect();
    assert_eq!(
        found.len(),
        2,
        "unpaired len and wrong-form is_empty both flagged: {found:?}"
    );
    assert!(
        types.contains(&"usize"),
        "len with no qualifying is_empty sibling flagged: {found:?}"
    );
    assert!(
        types.contains(&"bool"),
        "`&mut self` is_empty bool flagged: {found:?}"
    );
    assert!(
        found.iter().all(|v| v.kind == PositionKind::FnReturn),
        "both are return positions: {found:?}"
    );
}

/// Carve-out (b), POSITIVE — inside a `glam`-vector newtype's inherent `impl`, a
/// bare param/return of the inner vector's COMPONENT scalar is the coordinate
/// boundary: `Grid::new(i32, i32)` and the `i32` component accessor both clear.
#[test]
fn coordinate_component_scalar_is_exempt() {
    let found = scan(
        "\
/// A grid coordinate wrapping a glam IVec2.
pub struct Grid(IVec2);

impl Grid {
    /// Constructor boundary — i32 matches IVec2's component.
    pub const fn new(x: i32, y: i32) -> Self {
        Self(IVec2::new(x, y))
    }
    /// Component accessor — returns the i32 component.
    pub const fn x(&self) -> i32 {
        self.0.x
    }
}
",
    );
    assert!(found.is_empty(), "component scalars are exempt: {found:?}");
}

/// Carve-out (b), NEGATIVE — a scalar param on a NON-coordinate (scalar-inner)
/// newtype's constructor is flagged (rule 5 exempts only the EXACT inner), and a
/// return whose scalar does NOT match a `glam` newtype's component type is
/// flagged too.
#[test]
fn scalar_on_non_coordinate_newtype_is_flagged() {
    let weight = scan(
        "\
/// A scalar-inner newtype (inner is u32, not a glam vector).
pub struct Weight(u32);

impl Weight {
    /// An i32 param does NOT match the u32 inner — flagged.
    pub fn from_grams(grams: i32) -> Self {
        Self(grams.unsigned_abs())
    }
}
",
    );
    assert_eq!(
        weight.len(),
        1,
        "mismatched scalar param flagged: {weight:?}"
    );
    let Some(violation) = weight.first() else {
        unreachable!("one violation was asserted");
    };
    assert_eq!(&*violation.type_name, "i32");
    assert_eq!(violation.kind, PositionKind::FnParam);

    // A glam newtype, but the return scalar (f32) is NOT IVec2's component (i32).
    let mismatch = scan(
        "\
/// A grid coordinate wrapping IVec2 (component scalar i32).
pub struct Grid(IVec2);

impl Grid {
    /// f32 does NOT match IVec2's i32 component — flagged.
    pub fn ratio(&self) -> f32 {
        0.0
    }
}
",
    );
    assert_eq!(
        mismatch.len(),
        1,
        "wrong-component scalar flagged: {mismatch:?}"
    );
    let Some(violation) = mismatch.first() else {
        unreachable!("one violation was asserted");
    };
    assert_eq!(&*violation.type_name, "f32");
    assert_eq!(violation.kind, PositionKind::FnReturn);
}

/// Carve-out (c), POSITIVE — an `index() -> usize` on a type whose inherent
/// `impl` declares a fixed-size-array `const` (the collection it indexes) is the
/// provable own-collection index and is exempt.
#[test]
fn own_collection_index_is_exempt() {
    let found = scan(
        "\
/// A closed enum that owns a canonical-order array.
pub enum Part {
    Head,
    Torso,
}

impl Part {
    /// The owned collection this index keys into.
    pub const ALL: [Self; 2] = [Self::Head, Self::Torso];
    /// Provable own-collection index.
    pub const fn index(self) -> usize {
        match self {
            Self::Head => 0,
            Self::Torso => 1,
        }
    }
}
",
    );
    assert!(
        found.is_empty(),
        "the owned-array index is exempt: {found:?}"
    );
}

/// Carve-out (c), NEGATIVE — an `index() -> usize` on a type with NO owned array
/// `const` is still flagged (the checker cannot prove the collection).
#[test]
fn index_without_owned_array_is_flagged() {
    let found = scan(
        "\
/// A type with an index method but no owned collection const.
pub struct Cursor;

impl Cursor {
    /// No owned fixed-size-array const to prove — NOT exempt.
    pub fn index(&self) -> usize {
        0
    }
}
",
    );
    assert_eq!(found.len(), 1, "unprovable index flagged: {found:?}");
    let Some(violation) = found.first() else {
        unreachable!("one violation was asserted");
    };
    assert_eq!(&*violation.type_name, "usize");
    assert_eq!(violation.kind, PositionKind::FnReturn);
}

/// Carve-out (d), POSITIVE — a function whose name contains `hash` returning a
/// bare unsigned integer is a raw digest value and is exempt (case-insensitive).
#[test]
fn named_hash_digest_is_exempt() {
    let found = scan(
        "\
/// A free hashing function — its name says hash, its return is a digest.
pub fn hash_stream_seed(bytes: &[u8]) -> u64 {
    let _ = bytes;
    0
}

/// Case-insensitive: `Hash` still counts.
pub fn compute_Hash() -> usize {
    0
}
",
    );
    assert!(found.is_empty(), "named hash digests are exempt: {found:?}");
}

/// Carve-out (d), NEGATIVE — a fn whose name does NOT say `hash` returning an
/// unsigned int, and a `hash`-named fn returning a NON-unsigned type, are both
/// flagged (cast/sampling helpers are not covered).
#[test]
fn non_hash_and_non_unsigned_returns_are_flagged() {
    let checksum = scan(
        "\
/// Name does not contain `hash` — the u64 return is flagged.
pub fn checksum(bytes: &[u8]) -> u64 {
    let _ = bytes;
    0
}
",
    );
    assert_eq!(checksum.len(), 1, "non-hash u64 flagged: {checksum:?}");
    let Some(violation) = checksum.first() else {
        unreachable!("one violation was asserted");
    };
    assert_eq!(&*violation.type_name, "u64");

    let ratio = scan(
        "\
/// Name says hash but the return is f32, not an unsigned digest — flagged.
pub fn hash_ratio() -> f32 {
    0.0
}
",
    );
    assert_eq!(ratio.len(), 1, "hash-named f32 flagged: {ratio:?}");
    let Some(violation) = ratio.first() else {
        unreachable!("one violation was asserted");
    };
    assert_eq!(&*violation.type_name, "f32");
}
