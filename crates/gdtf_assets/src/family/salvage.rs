use std::path::{Path, PathBuf};

use bevy::{
    asset::{
        AssetPath, AssetServer, Assets, Handle, LoadState,
        io::{AssetReaderError, AssetSourceId, ErasedAssetReader},
    },
    ecs::resource::Resource,
    prelude::Deref,
    reflect::TypePath,
    tasks::{block_on, futures_lite::StreamExt},
};

use crate::{
    asset::RonAsset,
    family::report::{
        ContentFinding, ContentIntegrityReport, FindingDetail, FindingFamily, FindingReferrer,
    },
};

#[derive(Deref, Debug, Clone, PartialEq, Eq)]
pub struct SalvageMemberPath(String);

impl SalvageMemberPath {
    #[must_use]
    pub const fn new(path: String) -> Self {
        Self(path)
    }
}

#[derive(Deref, Debug, Clone, PartialEq, Eq)]
pub struct SalvageFolder(String);

impl SalvageFolder {
    #[must_use]
    pub const fn new(folder: String) -> Self {
        Self(folder)
    }
}

#[derive(Debug)]
struct SalvageMember<T>
where
    T: TypePath + Send + Sync + 'static,
{
    path:   SalvageMemberPath,
    handle: Handle<RonAsset<T>>,
}

#[derive(Resource, Debug)]
pub struct RonFolderSalvage<T>
where
    T: TypePath + Send + Sync + 'static,
{
    folder:  SalvageFolder,
    members: Vec<SalvageMember<T>>,
}

impl<T> RonFolderSalvage<T>
where
    T: TypePath + Send + Sync + 'static,
{
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.members.is_empty()
    }

    #[must_use]
    pub const fn folder(&self) -> &SalvageFolder {
        &self.folder
    }
}

pub struct SalvagedMember<'a, T>
where
    T: TypePath + Send + Sync + 'static,
{
    pub path: &'a SalvageMemberPath,
    pub spec: &'a RonAsset<T>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MalformedMember {
    pub path:  FindingReferrer,
    pub error: FindingDetail,
}

pub enum RonSalvagePoll<'a, T>
where
    T: TypePath + Send + Sync + 'static,
{
    Pending,
    Settled {
        loaded:    Vec<SalvagedMember<'a, T>>,
        malformed: Vec<MalformedMember>,
    },
}

/// Start loading every RON under `folder` with the given extension.
///
/// # Errors
///
/// Returns [`AssetReaderError`] when the default asset source is missing or the folder cannot be listed.
pub fn begin_ron_folder_salvage<T>(
    asset_server: &AssetServer,
    folder: &str,
    extension: &str,
) -> Result<RonFolderSalvage<T>, AssetReaderError>
where
    T: TypePath + Send + Sync + 'static,
{
    let paths = enumerate_member_paths(asset_server, folder, extension)?;
    let members = paths
        .into_iter()
        .map(|path| {
            let handle = asset_server.load::<RonAsset<T>>(AssetPath::parse(&path).into_owned());
            SalvageMember {
                path: SalvageMemberPath::new(path),
                handle,
            }
        })
        .collect();
    Ok(RonFolderSalvage {
        folder: SalvageFolder::new(folder.to_owned()),
        members,
    })
}

#[must_use]
pub fn poll_ron_folder_salvage<'a, T>(
    salvage: &'a RonFolderSalvage<T>,
    asset_server: &AssetServer,
    specs: &'a Assets<RonAsset<T>>,
) -> RonSalvagePoll<'a, T>
where
    T: TypePath + Send + Sync + 'static,
{
    let mut loaded = Vec::new();
    let mut malformed = Vec::new();
    for member in &salvage.members {
        match asset_server.load_state(member.handle.id()) {
            LoadState::Failed(error) => malformed.push(MalformedMember {
                path:  FindingReferrer::new(member.path.as_str().to_owned()),
                error: FindingDetail::new(error.to_string()),
            }),
            LoadState::Loaded => {
                let Some(spec) = specs.get(&member.handle) else {
                    return RonSalvagePoll::Pending;
                };
                loaded.push(SalvagedMember {
                    path: &member.path,
                    spec,
                });
            }
            LoadState::NotLoaded | LoadState::Loading => return RonSalvagePoll::Pending,
        }
    }
    RonSalvagePoll::Settled { loaded, malformed }
}

#[must_use]
pub fn salvage_members_for_rebuild<'a, T>(
    salvage: &'a RonFolderSalvage<T>,
    asset_server: &AssetServer,
    specs: &'a Assets<RonAsset<T>>,
) -> Option<Vec<SalvagedMember<'a, T>>>
where
    T: TypePath + Send + Sync + 'static,
{
    match poll_ron_folder_salvage(salvage, asset_server, specs) {
        RonSalvagePoll::Pending => None,
        RonSalvagePoll::Settled { loaded, .. } => Some(loaded),
    }
}

pub fn report_malformed_members(
    report: Option<&mut ContentIntegrityReport>,
    family: &FindingFamily,
    malformed: Vec<MalformedMember>,
) {
    let mut report = report;
    for member in malformed {
        bevy::log::warn!(
            "GDTF Load: `{}` failed to load into {} ({}); its well-formed \
             siblings were salvaged per-file",
            &*member.path,
            &**family,
            &*member.error,
        );
        if let Some(report) = report.as_deref_mut() {
            report.record(ContentFinding::MalformedFile {
                path:   member.path,
                family: family.clone(),
                detail: member.error,
            });
        }
    }
}

fn enumerate_member_paths(
    asset_server: &AssetServer,
    folder: &str,
    extension: &str,
) -> Result<Vec<String>, AssetReaderError> {
    let source = asset_server
        .get_source(AssetSourceId::Default)
        .map_err(|_| AssetReaderError::NotFound(PathBuf::from(folder)))?;
    let reader = source.reader();
    let suffix = format!(".{extension}");
    let mut paths = Vec::new();
    block_on(collect_members(
        reader,
        Path::new(folder),
        &suffix,
        &mut paths,
    ))?;
    paths.sort();
    Ok(paths)
}

async fn collect_members(
    reader: &dyn ErasedAssetReader,
    path: &Path,
    suffix: &str,
    out: &mut Vec<String>,
) -> Result<(), AssetReaderError> {
    let mut stream = reader.read_directory(path).await?;
    while let Some(child) = stream.next().await {
        if reader.is_directory(&child).await? {
            Box::pin(collect_members(reader, &child, suffix, out)).await?;
        } else {
            let child_path = child.to_string_lossy().replace('\\', "/");
            if child_path.ends_with(suffix) {
                out.push(child_path);
            }
        }
    }
    Ok(())
}
