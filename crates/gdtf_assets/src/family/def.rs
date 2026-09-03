//! Trait describing a RON content family and its registry.

use bevy::{
    ecs::{component::Mutable, resource::Resource},
    prelude::Deref,
    reflect::TypePath,
};
use serde::Deserialize;

/// A folder of RON files that rebuilds a registry resource.
pub trait ContentFamily: Send + Sync + 'static {
    /// Deserialized member type.
    type Spec: for<'de> Deserialize<'de> + TypePath + Send + Sync + 'static;

    /// Registry resource filled from members.
    type Registry: Resource<Mutability = Mutable> + Default;

    /// Asset folder path.
    const FOLDER: &'static str;

    /// File extension (without leading dot).
    const EXTENSION: &'static str;

    /// Insert one member into the registry, answering the key it was inserted under.
    ///
    /// `None` means no source path is recorded for this member: the family inserted
    /// nothing, or it keys by something no single string names.
    fn insert_member(
        registry: &mut Self::Registry,
        stem: Option<ContentFileStem>,
        spec: &Self::Spec,
    ) -> Option<ContentMemberKey>;
}

/// The key a family inserted a member under, as text.
#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash)]
pub struct ContentMemberKey(String);

impl ContentMemberKey {
    /// Wrap a member key.
    #[must_use]
    pub const fn new(key: String) -> Self {
        Self(key)
    }
}

/// File stem used as a registry key when present.
#[derive(Deref, Debug, Clone, PartialEq, Eq)]
pub struct ContentFileStem(String);

impl ContentFileStem {
    /// Wrap a stem string.
    #[must_use]
    pub const fn new(stem: String) -> Self {
        Self(stem)
    }

    /// Consume into the inner string.
    #[must_use]
    pub fn into_inner(self) -> String {
        self.0
    }
}
