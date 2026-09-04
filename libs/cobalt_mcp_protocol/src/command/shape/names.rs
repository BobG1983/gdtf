//! Type and field names used inside a shape document.

use std::borrow::Cow;

use bevy_derive::Deref;
use serde::{Deserialize, Serialize};

/// Name of a type in a shape document.
#[derive(Deref, Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ShapeName(Cow<'static, str>);

impl ShapeName {
    /// Borrow a static type name.
    #[must_use]
    pub const fn from_static(name: &'static str) -> Self {
        Self(Cow::Borrowed(name))
    }

    /// Own a dynamic type name.
    #[must_use]
    pub const fn from_owned(name: String) -> Self {
        Self(Cow::Owned(name))
    }

    /// Borrow as `&str`.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Name of one field inside a record shape.
#[derive(Deref, Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ShapeFieldName(Cow<'static, str>);

impl ShapeFieldName {
    /// Borrow a static field name.
    #[must_use]
    pub const fn from_static(name: &'static str) -> Self {
        Self(Cow::Borrowed(name))
    }

    /// Own a dynamic field name.
    #[must_use]
    pub const fn from_owned(name: String) -> Self {
        Self(Cow::Owned(name))
    }

    /// Borrow as `&str`.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
