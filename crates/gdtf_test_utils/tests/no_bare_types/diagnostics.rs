//! Clippy-style rendering of a [`Violation`] and its stable registry key.

use crate::types::Violation;

impl Violation {
    /// The `path:line:col` source location string.
    pub(crate) fn location(&self) -> String {
        format!("{}:{}:{}", *self.path, *self.line, *self.column)
    }

    /// The registry key — `path:line:col type` — matched against the exemption
    /// file. Stable as long as the offending source line does not move.
    pub(crate) fn key(&self) -> String {
        format!("{} {}", self.location(), *self.type_name)
    }

    /// A clippy-style two-line diagnostic:
    /// ```text
    /// no-bare-types: bare `u32` used as a domain value (struct field)
    ///   --> crates/foo/src/bar.rs:42:9
    /// ```
    pub(crate) fn render(&self) -> String {
        format!(
            "no-bare-types: bare `{}` used as a domain value ({})\n  --> {}",
            *self.type_name,
            self.kind.phrase(),
            self.location()
        )
    }
}
