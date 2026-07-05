//! Tests for the theme schema and spec→runtime resolution, split by concern:
//! the shipped-file smoke test, the field-mapping mechanism, the font
//! override-vs-default selection, and the GTW-325 newtype privacy guard.

mod fonts;
mod mapping;
mod privacy;
mod shipped;
