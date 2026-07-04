//! The one-call app registration extension for folder-loaded content families.

use bevy::{app::App, asset::AssetServer, prelude::*};

use crate::{
    ext::RonAssetAppExt,
    family::{
        def::ContentFamily,
        handle::ContentFolderHandle,
        report::ContentIntegrityReport,
        systems::{kick_off_content_family, redrive_content_family, resolve_content_family},
    },
};

/// One-call registration of a folder-loaded content family — the GTW-570 seam.
///
/// [`register_content_family`](Self::register_content_family) wires ONE
/// family: the dedicated-extension [`RonAsset`](crate::RonAsset) loader, a
/// `Startup` kick-off that starts the recursive folder load and stores the
/// persistent [`ContentFolderHandle`], an `Update` resolve gated to insert the
/// registry exactly once (fail-closed to an EMPTY registry on a genuine
/// `Failed`), and an ungated `Update` redrive that rebuilds it live on a
/// member `Modified` event. An eighth folder family is one
/// [`ContentFamily`] marker impl plus one of these calls plus a content
/// folder — see the trait's add-one-family recipe.
///
/// The Load→Intro gating stays the HOST's: this seam publishes the registry
/// resource; the host's transition run-condition chain keeps requiring it
/// explicitly (`resource_exists::<F::Registry>`), exactly as before (GTW-570
/// C3).
pub trait ContentFamilyAppExt {
    /// Registers a content family's loader + kick-off / resolve / redrive
    /// chain.
    ///
    /// Registration SELF-GATES on an [`AssetServer`] being present
    /// (registering an asset without one panics), so a `MinimalPlugins`
    /// headless app skips the whole chain — no loader, no systems, no panic
    /// (`bevy-traps.md` #1) — the [`HotRonAppExt`](crate::HotRonAppExt)
    /// precedent.
    fn register_content_family<F: ContentFamily>(&mut self) -> &mut Self;
}

impl ContentFamilyAppExt for App {
    fn register_content_family<F: ContentFamily>(&mut self) -> &mut Self {
        if self.world().get_resource::<AssetServer>().is_none() {
            return self;
        }
        // The DEDICATED compound extension keeps the untyped `load_folder`
        // dispatch unambiguous among GDTF's many `.ron` loaders (GTW-257).
        self.init_ron_asset_with_extensions::<F::Spec>(vec![F::EXTENSION]);
        // GTW-582 C4: the per-file salvage records each malformed member into
        // the content-integrity report, so every seam host carries one
        // (idempotent init — the game's Load plugin installs the full
        // validation pass on top; the editor gets the report alone).
        self.init_resource::<ContentIntegrityReport>();
        self.add_systems(Startup, kick_off_content_family::<F>)
            .add_systems(
                Update,
                // Insert-exactly-once + the own-absence shadow semantics
                // headless seeds rely on (the GTW-564 resolve-gate precedent).
                resolve_content_family::<F>.run_if(
                    resource_exists::<ContentFolderHandle<F>>
                        .and_then(not(resource_exists::<F::Registry>)),
                ),
            )
            .add_systems(Update, redrive_content_family::<F>);
        self
    }
}
