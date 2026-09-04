use bevy::{asset::LoadedFolder, prelude::*};
use cobalt_ron_assets::HotRonHandle;
use gdtf_ui::theme::GdtfThemeSpec;

#[derive(Deref, Clone, Debug)]
pub(in crate::states::load) struct FontFolderHandle(Handle<LoadedFolder>);

impl FontFolderHandle {
    pub(in crate::states::load) const fn new(handle: Handle<LoadedFolder>) -> Self {
        Self(handle)
    }
}

#[derive(Deref, Clone, Debug)]
pub(in crate::states::load) struct InjuriesFolderHandle(Handle<LoadedFolder>);

impl InjuriesFolderHandle {
    pub(in crate::states::load) const fn new(handle: Handle<LoadedFolder>) -> Self {
        Self(handle)
    }
}

#[derive(Deref, Clone, Debug)]
pub(in crate::states::load) struct PrefabsFolderHandle(Handle<LoadedFolder>);

impl PrefabsFolderHandle {
    pub(in crate::states::load) const fn new(handle: Handle<LoadedFolder>) -> Self {
        Self(handle)
    }
}

#[derive(Resource, Clone, Debug)]
pub(in crate::states::load) struct LoadHandles {
    pub theme:    HotRonHandle<GdtfThemeSpec>,
    pub fonts:    FontFolderHandle,
    pub injuries: InjuriesFolderHandle,
    pub prefabs:  PrefabsFolderHandle,
}

#[derive(Resource, Deref, Clone, Debug)]
pub(in crate::states::load) struct ActiveInjuriesFolderHandle(Handle<LoadedFolder>);

impl ActiveInjuriesFolderHandle {
    pub(in crate::states::load) const fn new(handle: Handle<LoadedFolder>) -> Self {
        Self(handle)
    }
}

#[derive(Resource, Deref, Clone, Debug)]
pub(in crate::states::load) struct ActivePrefabsFolderHandle(Handle<LoadedFolder>);

impl ActivePrefabsFolderHandle {
    pub(in crate::states::load) const fn new(handle: Handle<LoadedFolder>) -> Self {
        Self(handle)
    }
}

#[derive(Deref, Clone, PartialEq, Eq, Debug)]
pub(in crate::states::load) struct FailedAssetPath(String);

impl FailedAssetPath {
    pub(in crate::states::load) fn new(path: impl Into<String>) -> Self {
        Self(path.into())
    }
}

#[derive(Resource, Deref, Clone, PartialEq, Eq, Debug)]
pub(in crate::states::load) struct LoadFailed(FailedAssetPath);

impl LoadFailed {
    pub(in crate::states::load) const fn new(path: FailedAssetPath) -> Self {
        Self(path)
    }
}
