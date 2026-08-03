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

pub trait ContentFamilyAppExt {
                                                    fn register_content_family<F: ContentFamily>(&mut self) -> &mut Self;
}

impl ContentFamilyAppExt for App {
    fn register_content_family<F: ContentFamily>(&mut self) -> &mut Self {
        if self.world().get_resource::<AssetServer>().is_none() {
            self.init_resource::<F::Registry>();
            return self;
        }
        self.init_ron_asset_with_extensions::<F::Spec>(vec![F::EXTENSION]);
        self.init_resource::<ContentIntegrityReport>();
        self.add_systems(Startup, kick_off_content_family::<F>)
            .add_systems(
                Update,
                resolve_content_family::<F>.run_if(
                    resource_exists::<ContentFolderHandle<F>>
                        .and_then(not(resource_exists::<F::Registry>)),
                ),
            )
            .add_systems(Update, redrive_content_family::<F>);
        self
    }
}
