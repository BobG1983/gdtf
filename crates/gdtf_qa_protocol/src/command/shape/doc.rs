//! The vocabulary a published shape document is written in.

use serde::{Deserialize, Serialize};

use super::names::{ShapeFieldName, ShapeName};

/// One RON value's shape.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RonShape {
    /// The empty value `()`.
    Unit,
    /// `true` or `false`.
    Bool,
    /// A whole number.
    Int,
    /// A fractional number.
    Float,
    /// A quoted string.
    Text,
    /// A byte string.
    Bytes,
    /// `Some(value)` or `None`.
    Optional(Box<Self>),
    /// `[a,b,c]`.
    List(Box<Self>),
    /// `{key:value}`.
    Map(Box<Self>, Box<Self>),
    /// `(a,b)`.
    Tuple(Vec<Self>),
    /// A named type; look it up in the document's definitions.
    Named(ShapeName),
}

/// One named field of a record shape.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ShapeField(ShapeFieldName, RonShape);

impl ShapeField {
    /// Pair a field name with its shape.
    #[must_use]
    pub const fn new(name: ShapeFieldName, shape: RonShape) -> Self {
        Self(name, shape)
    }

    /// Field name.
    #[must_use]
    pub const fn name(&self) -> &ShapeFieldName {
        &self.0
    }

    /// Field shape.
    #[must_use]
    pub const fn shape(&self) -> &RonShape {
        &self.1
    }
}

/// The body one enum variant carries.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VariantShape {
    /// `Name`.
    Unit,
    /// `Name(value)`.
    Newtype(RonShape),
    /// `Name(a,b)`.
    Tuple(Vec<RonShape>),
    /// `Name(field:value)`.
    Record(Vec<ShapeField>),
}

/// One variant of a choice shape.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ShapeVariant(ShapeName, VariantShape);

impl ShapeVariant {
    /// Pair a variant name with its body.
    #[must_use]
    pub const fn new(name: ShapeName, body: VariantShape) -> Self {
        Self(name, body)
    }

    /// Variant name.
    #[must_use]
    pub const fn name(&self) -> &ShapeName {
        &self.0
    }

    /// Variant body.
    #[must_use]
    pub const fn body(&self) -> &VariantShape {
        &self.1
    }
}

/// What a named type looks like on the wire.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ShapeBody {
    /// A unit struct, or a newtype or tuple struct that is not transparent.
    Wraps(RonShape),
    /// A struct with named fields.
    Record(Vec<ShapeField>),
    /// An enum.
    Choice(Vec<ShapeVariant>),
}

/// One named type's definition.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ShapeDef(ShapeName, ShapeBody);

impl ShapeDef {
    /// Pair a type name with its body.
    #[must_use]
    pub const fn new(name: ShapeName, body: ShapeBody) -> Self {
        Self(name, body)
    }

    /// Type name.
    #[must_use]
    pub const fn name(&self) -> &ShapeName {
        &self.0
    }

    /// Type body.
    #[must_use]
    pub const fn body(&self) -> &ShapeBody {
        &self.1
    }
}

/// A published shape: one root value plus every named type it reaches.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ShapeDoc {
    root: RonShape,
    defs: Vec<ShapeDef>,
}

impl ShapeDoc {
    /// Build a document from its root shape and definitions.
    #[must_use]
    pub const fn new(root: RonShape, defs: Vec<ShapeDef>) -> Self {
        Self { root, defs }
    }

    /// The shape of the value itself.
    #[must_use]
    pub const fn root(&self) -> &RonShape {
        &self.root
    }

    /// Every named type the root reaches.
    #[must_use]
    pub fn defs(&self) -> &[ShapeDef] {
        &self.defs
    }

    /// Look one named type up in the definitions.
    #[must_use]
    pub fn body_of(&self, name: &ShapeName) -> Option<&ShapeBody> {
        self.defs
            .iter()
            .find(|def| def.name() == name)
            .map(ShapeDef::body)
    }

    /// The body the root resolves to, when the root is a named type.
    #[must_use]
    pub fn root_body(&self) -> Option<&ShapeBody> {
        match &self.root {
            RonShape::Named(name) => self.body_of(name),
            _ => None,
        }
    }
}
