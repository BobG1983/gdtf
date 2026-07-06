//! The [`ContentFamily`] trait — the per-family contract the generic
//! folder-to-registry chain is instantiated over.

use bevy::{
    ecs::{component::Mutable, resource::Resource},
    prelude::Deref,
    reflect::TypePath,
};
use serde::Deserialize;

/// One folder-loaded content family: a folder of loose `.ron` files resolved
/// into a single registry [`Resource`] (GTW-570).
///
/// # Adding a new content family
///
/// A new family costs exactly three things — no per-family resolve module, no
/// poll branch, no handle newtype, no redrive clone:
///
/// 1. **One marker impl** of this trait naming the family's payload
///    ([`Spec`](ContentFamily::Spec)), registry
///    ([`Registry`](ContentFamily::Registry)), folder, extension, and how a
///    member keys into the registry ([`insert_member`](ContentFamily::insert_member)).
/// 2. **One registration line** in the host app:
///    `app.register_content_family::<MyFamily>()`
///    (see [`ContentFamilyAppExt`](crate::ContentFamilyAppExt)).
/// 3. **A content folder** of `<key>.<infix>.ron` files under the asset root.
///
/// # The two keying shapes
///
/// The ONLY axis the shipped families vary on is where a member's
/// registry key comes from; both shapes are expressed through
/// [`insert_member`](ContentFamily::insert_member), never a mode flag:
///
/// - **Stem-keyed** — the key is the member's file stem with the family's
///   dedicated infix stripped (`stub_pistol.weapon.ron` → `stub_pistol`). The
///   impl keys by the provided [`ContentFileStem`], skipping a member whose
///   handle carries no resolvable path (it would have no usable key).
/// - **Payload-keyed** — the key (a UUID) lives INSIDE the deserialized
///   payload (the terrain / theme defs). The impl ignores the stem and reads
///   the key off `spec`.
///
/// # What the generic chain guarantees (encoded once, per GTW-570 C2)
///
/// - headless (`Option<Res<AssetServer>>`) no-op — a `MinimalPlugins` app
///   registers nothing and never panics;
/// - fail-closed on a `Failed` folder — `warn!` + an EMPTY registry (ADR-0003),
///   so a presence-gated `Load` flow is never stranded;
/// - never-publish-partial — while ANY matching-type member is absent from its
///   `Assets` collection the resolve returns and retries next frame;
/// - a persistent [`ContentFolderHandle`](crate::ContentFolderHandle) beside
///   the registry, keeping every member loaded for the file-watcher;
/// - a redrive that rebuilds the registry in place on a member
///   [`AssetEvent::Modified`](bevy::asset::AssetEvent), logging the reload
///   (the GTW-374 Part C convention);
/// - an UNCONDITIONAL `TypeId` member filter, so a mixed folder (terrain defs
///   and theme defs share one tree) never mistypes a member.
pub trait ContentFamily: Send + Sync + 'static {
    /// The payload each member `.ron` file deserializes into (loaded as a
    /// [`RonAsset<Spec>`](crate::RonAsset)).
    type Spec: for<'de> Deserialize<'de> + TypePath + Send + Sync + 'static;

    /// The registry resource the folder resolves into. [`Default`] is the
    /// EMPTY registry the fail-closed path publishes; `Mutability = Mutable`
    /// lets the redrive overwrite it through `ResMut` (change detection).
    type Registry: Resource<Mutability = Mutable> + Default;

    /// The family's content folder, relative to the asset source root
    /// (e.g. `content/weapons/ranged`).
    const FOLDER: &'static str;

    /// The family's DEDICATED compound extension (e.g. `weapon.ron`), keeping
    /// Bevy's extension-based `load_folder` dispatch unambiguous among GDTF's
    /// many `.ron` loaders (the GTW-257 precedent).
    const EXTENSION: &'static str;

    /// Fold ONE loaded member into the registry under construction.
    ///
    /// `stem` is the member's file stem with the family's dedicated infix
    /// already stripped, or [`None`] when the member handle carries no
    /// resolvable path. A stem-keyed family keys by it (skipping a stem-less
    /// member); a payload-keyed family ignores it and keys by the id inside
    /// `spec`. Called once per matching-type member by BOTH the one-time
    /// resolve and the live redrive, so they build identically.
    fn insert_member(
        registry: &mut Self::Registry,
        stem: Option<ContentFileStem>,
        spec: &Self::Spec,
    );
}

/// A content-family member's KEY STEM: its file stem with the family's
/// dedicated infix stripped (`stub_pistol.weapon.ron` → `stub_pistol`).
///
/// A named newtype over the stem `String` (no-bare-types rule); read through
/// [`Deref`] or consumed via [`into_inner`](ContentFileStem::into_inner) to
/// build the family's key newtype.
#[derive(Deref, Debug, Clone, PartialEq, Eq)]
pub struct ContentFileStem(String);

impl ContentFileStem {
    /// Wrap an already-stripped member key stem.
    #[must_use]
    pub const fn new(stem: String) -> Self {
        Self(stem)
    }

    /// Consume the wrapper, yielding the owned stem — the shape a family's
    /// key-newtype constructor (e.g. `WeaponName::new`) takes.
    #[must_use]
    pub fn into_inner(self) -> String {
        self.0
    }
}
