use bevy::{
    ecs::{component::Mutable, resource::Resource},
    prelude::Deref,
    reflect::TypePath,
};
use serde::Deserialize;

pub trait ContentFamily: Send + Sync + 'static {
            type Spec: for<'de> Deserialize<'de> + TypePath + Send + Sync + 'static;

                type Registry: Resource<Mutability = Mutable> + Default;

            const FOLDER: &'static str;

                const EXTENSION: &'static str;

                                    fn insert_member(
        registry: &mut Self::Registry,
        stem: Option<ContentFileStem>,
        spec: &Self::Spec,
    );
}

#[derive(Deref, Debug, Clone, PartialEq, Eq)]
pub struct ContentFileStem(String);

impl ContentFileStem {
        #[must_use]
    pub const fn new(stem: String) -> Self {
        Self(stem)
    }

            #[must_use]
    pub fn into_inner(self) -> String {
        self.0
    }
}
