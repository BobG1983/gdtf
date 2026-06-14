//! Loose-file RON asset loading for GDTF.
//!
//! GDTF ships its data-driven content (the UI theme, and later content tables)
//! as **loose `.ron` files under the workspace-root `assets/` directory** (ADR
//! 0003; embedding is deferred to packaging). This crate provides the one piece
//! every such file needs: a generic Bevy [`AssetLoader`] that reads a `.ron`
//! file's bytes and `serde`-deserializes them into a typed payload `T`.
//!
//! # The pieces
//!
//! - [`RonAsset<T>`] — a Bevy [`Asset`] newtype wrapping a deserialized `T`. It
//!   `Deref`s to `T`, so a consumer reads the payload directly off the loaded
//!   asset. `T` is any `serde`-`Deserialize` + `TypePath` value; the loader is
//!   generic over it, so one loader serves every RON content type.
//! - [`RonAssetLoader<T>`] — the [`AssetLoader`] impl. It reads the file bytes
//!   and `ron::de::from_bytes`-deserializes them into `T`, wrapping the result
//!   in [`RonAsset`]. A malformed file surfaces as a **typed** [`RonLoadError`]
//!   (which Bevy records as a failed load), never a panic or `unwrap`.
//! - [`RonLoadError`] — the loader's typed error: either the bytes could not be
//!   read, or the RON did not deserialize into `T`.
//! - [`RonAssetAppExt`] — a one-call registration extension: `app
//!   .init_ron_asset::<T>()` registers `Assets<RonAsset<T>>` and the loader for
//!   `T` in a single step.
//!
//! # Where the asset source root is
//!
//! The Bevy file [`AssetReader`](bevy::asset::io::AssetReader) resolves loose
//! paths against a base directory chosen by `bevy_asset` as: the `BEVY_ASSET_ROOT`
//! env var if set, else `CARGO_MANIFEST_DIR` if set, else the running
//! executable's directory. For the production binary (`grimdark_turfwar`) Bevy's
//! default [`AssetPlugin`](bevy::asset::AssetPlugin) is used unchanged: `cargo
//! run` sets the **working directory to the workspace root** and Bevy resolves
//! against the binary, so `assets/...` lands on the repo-root `assets/`
//! directory. Headless tests cannot rely on that working directory — under
//! `cargo test`, `CARGO_MANIFEST_DIR` points at the *test crate*, not the repo
//! root — so the test harness
//! ([`GdtfUiTestAppBuilder`](../gdtf_test_utils/index.html)) explicitly points
//! [`AssetPlugin::file_path`](bevy::asset::AssetPlugin::file_path) at the
//! workspace-root `assets/` directory. Either way the source root is the **same**
//! repo-root `assets/`, so a path that loads in a test loads in the app.
//!
//! This crate does **not** configure the asset source root itself — it loads
//! whatever the host app's `AssetPlugin` is pointed at — so it stays a pure,
//! reusable leaf that both GTW-56 (`Load`) and GTW-42 (content) can depend on.

use core::marker::PhantomData;
use std::fmt;

use bevy::{
    app::App,
    asset::{Asset, AssetApp, AssetLoader, LoadContext, io::Reader},
    reflect::TypePath,
};
use serde::Deserialize;

/// A loaded RON asset: a typed payload `T` deserialized from a loose `.ron` file.
///
/// This is the [`Asset`] the [`RonAssetLoader`] produces. It is a transparent
/// newtype over `T` and [`Deref`](std::ops::Deref)s to it, so a consumer that
/// holds a `RonAsset<MyType>` reads the `MyType` fields straight through.
///
/// `T` must be a `serde`-`Deserialize` value that is also `TypePath`, `Send`,
/// `Sync`, and `'static` (the bounds Bevy's asset system requires of any asset
/// payload). The loose-file path the asset was loaded from (e.g.
/// `theme/grimdark.ron`) is resolved by the host app's
/// `AssetServer` against its configured source root.
#[derive(Asset, TypePath)]
pub struct RonAsset<T>(pub T)
where
    T: TypePath + Send + Sync + 'static;

impl<T> core::ops::Deref for RonAsset<T>
where
    T: TypePath + Send + Sync + 'static,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// The typed error a [`RonAssetLoader`] can fail with.
///
/// A malformed or unreadable file fails the load with one of these variants —
/// never a panic. Bevy records the failure on the asset's load state (it
/// becomes `Failed`) and surfaces the error in the asset events, so a consumer
/// can react to a bad file rather than crashing on it.
#[derive(Debug)]
pub enum RonLoadError {
    /// The file bytes could not be read from the asset source.
    Read(ReadError),
    /// The bytes were read but did not deserialize into the target type as RON.
    Deserialize(RonDeError),
}

/// The underlying I/O error from reading a RON file's bytes off the asset source.
///
/// A named newtype over [`std::io::Error`] so the domain error
/// [`RonLoadError::Read`] carries a typed cause rather than a bare std type.
#[derive(Debug)]
pub struct ReadError(std::io::Error);

impl core::ops::Deref for ReadError {
    type Target = std::io::Error;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// The underlying RON deserialization error (with source span) from parsing a
/// RON file into the target type.
///
/// A named newtype over [`ron::error::SpannedError`] so the domain error
/// [`RonLoadError::Deserialize`] carries a typed cause rather than a bare
/// foreign type.
#[derive(Debug)]
pub struct RonDeError(ron::error::SpannedError);

impl core::ops::Deref for RonDeError {
    type Target = ron::error::SpannedError;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl fmt::Display for RonLoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read(err) => write!(f, "could not read RON asset bytes: {}", err.0),
            Self::Deserialize(err) => write!(f, "could not deserialize RON asset: {}", err.0),
        }
    }
}

impl std::error::Error for RonLoadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Read(err) => Some(&err.0),
            Self::Deserialize(err) => Some(&err.0),
        }
    }
}

/// A generic [`AssetLoader`] that reads a loose `.ron` file and deserializes it
/// into a [`RonAsset`] wrapping `T`.
///
/// One loader type serves every RON content type: register it per payload with
/// [`RonAssetAppExt::init_ron_asset`]. The loader is stateless (a zero-sized
/// marker over `T`), so [`Default`] constructs it and `init_asset_loader`
/// registers it with no settings.
///
/// On `load`, it reads the entire file into a buffer and calls
/// `ron::de::from_bytes::<T>`. Any read or parse failure becomes a typed
/// [`RonLoadError`]; the happy path never panics or `unwrap`s.
///
/// It derives [`TypePath`] (required of every [`AssetLoader`]); the derive needs
/// `T: TypePath`, which every RON payload already satisfies via [`RonAsset`].
#[derive(TypePath)]
pub struct RonAssetLoader<T>
where
    T: TypePath,
{
    /// Zero-sized type marker; the loader carries no runtime state.
    _payload: PhantomData<fn() -> T>,
}

impl<T> Default for RonAssetLoader<T>
where
    T: TypePath,
{
    fn default() -> Self {
        Self {
            _payload: PhantomData,
        }
    }
}

impl<T> AssetLoader for RonAssetLoader<T>
where
    T: for<'de> Deserialize<'de> + TypePath + Send + Sync + 'static,
{
    type Asset = RonAsset<T>;
    type Settings = ();
    type Error = RonLoadError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &Self::Settings,
        _load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader
            .read_to_end(&mut bytes)
            .await
            .map_err(|err| RonLoadError::Read(ReadError(err)))?;
        let value = ron::de::from_bytes::<T>(&bytes)
            .map_err(|err| RonLoadError::Deserialize(RonDeError(err)))?;
        Ok(RonAsset(value))
    }

    fn extensions(&self) -> &[&str] {
        &["ron"]
    }
}

/// One-call registration of the RON loader for a payload type `T`.
///
/// Bundles the two registrations a RON content type needs — the
/// `Assets<RonAsset<T>>` collection and the [`RonAssetLoader<T>`] — so a host
/// crate adds support for a new RON type in a single line.
pub trait RonAssetAppExt {
    /// Registers `Assets<RonAsset<T>>` and the [`RonAssetLoader<T>`] for `T`.
    ///
    /// After this, `asset_server.load::<RonAsset<T>>("path/to/file.ron")` loads
    /// and deserializes a loose `.ron` file into a `RonAsset<T>`.
    fn init_ron_asset<T>(&mut self) -> &mut Self
    where
        T: for<'de> Deserialize<'de> + TypePath + Send + Sync + 'static;
}

impl RonAssetAppExt for App {
    fn init_ron_asset<T>(&mut self) -> &mut Self
    where
        T: for<'de> Deserialize<'de> + TypePath + Send + Sync + 'static,
    {
        self.init_asset::<RonAsset<T>>();
        self.init_asset_loader::<RonAssetLoader<T>>();
        self
    }
}
