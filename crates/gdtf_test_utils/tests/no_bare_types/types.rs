//! The domain vocabulary of the checker — named newtypes for the values that
//! flow through the walk and into diagnostics (repo paths, 1-based source
//! coordinates, rendered type names), the checked-position kinds, a single
//! rendered `Violation`, and the flagged bare-type set.
//!
//! This suite eats its own dog food (GTW-599): every domain value here is a
//! named newtype with a private inner. Pure plumbing inside the walker (syn AST
//! nodes, loop counters) rides rule 4 of `.claude/rules/no-bare-types.md`.

use bevy::prelude::Deref;

/// A repo-relative source path (forward slashes), e.g. `crates/foo/src/bar.rs`.
#[derive(Deref, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub(crate) struct RepoPath(String);

impl RepoPath {
    /// Wrap a repo-relative path string.
    pub(crate) fn new(path: impl Into<String>) -> Self {
        Self(path.into())
    }
}

/// A 1-based source line number (as syn/proc-macro2 report spans).
#[derive(Deref, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub(crate) struct LineNumber(usize);

impl LineNumber {
    /// Wrap a 1-based line number.
    pub(crate) const fn new(line: usize) -> Self {
        Self(line)
    }
}

/// A 1-based source column number.
#[derive(Deref, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub(crate) struct ColumnNumber(usize);

impl ColumnNumber {
    /// Wrap a 1-based column number.
    pub(crate) const fn new(column: usize) -> Self {
        Self(column)
    }
}

/// A rendered Rust type name — the last path segment of the offending type
/// (`u32`, `String`, `Vec3`, …).
#[derive(Deref, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub(crate) struct TypeName(String);

impl TypeName {
    /// Wrap a rendered type-name string.
    pub(crate) fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    /// Whether this type name is exactly `other` — used by the convention
    /// carve-out predicates to match a signature's flagged type against the
    /// std-container / index / hash shapes named in `.claude/rules/no-bare-types.md`.
    pub(crate) fn is(&self, other: &str) -> bool {
        self.0 == other
    }
}

/// Where in the source a bare type was found — shapes the diagnostic wording.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum PositionKind {
    /// A field of a `struct`.
    StructField,
    /// A field of an `enum` variant.
    EnumField,
    /// A function/method parameter.
    FnParam,
    /// A function/method return type.
    FnReturn,
}

impl PositionKind {
    /// The human phrase used in the diagnostic ("struct field", …).
    pub(crate) const fn phrase(self) -> &'static str {
        match self {
            Self::StructField => "struct field",
            Self::EnumField => "enum variant field",
            Self::FnParam => "function parameter",
            Self::FnReturn => "function return",
        }
    }
}

/// One flagged bare-type use — the offending type, where it sits, and its
/// source coordinate. Rendered clippy-style by `crate::diagnostics`.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) struct Violation {
    /// Repo-relative file the bare type was found in.
    pub(crate) path:      RepoPath,
    /// 1-based line of the offending type.
    pub(crate) line:      LineNumber,
    /// 1-based column of the offending type.
    pub(crate) column:    ColumnNumber,
    /// The bare type (last path segment).
    pub(crate) type_name: TypeName,
    /// The checked position the type appeared in.
    pub(crate) kind:      PositionKind,
}

/// Whether `name` (a type's last path segment) is a flagged bare type — a
/// primitive, `String`/`str`, a `glam` vector/quat, or a `std::time` clock
/// value. These carry domain meaning and must be wrapped in a named newtype
/// (`.claude/rules/no-bare-types.md`).
pub(crate) fn is_flagged(name: &str) -> bool {
    matches!(
        name,
        "u8" | "u16"
            | "u32"
            | "u64"
            | "u128"
            | "usize"
            | "i8"
            | "i16"
            | "i32"
            | "i64"
            | "i128"
            | "isize"
            | "f32"
            | "f64"
            | "bool"
            | "char"
            | "String"
            | "str"
            | "IVec2"
            | "IVec3"
            | "IVec4"
            | "UVec2"
            | "UVec3"
            | "UVec4"
            | "Vec2"
            | "Vec3"
            | "Vec4"
            | "Quat"
            | "Duration"
            | "Instant"
    )
}
