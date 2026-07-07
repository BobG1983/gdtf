//! The editor's `Load`-pass registration of the bespoke INJURIES family
//! (GTW-654) — one folder ([`INJURIES_FOLDER`]) resolving into TWO resources
//! (the [`InjuryRegistry`] + the [`InjuryTables`]), which is why it cannot ride
//! the generic GTW-570 `register_content_family` seam (a declared seam
//! exclusion).
//!
//! The folder-walk BUILDER + per-file salvage fold are the host-agnostic
//! [`gdtf_content_families::injuries`] halves the game's Load resolve also
//! runs (the GTW-630 "one source, two hosts" shape), so an authored injury /
//! weighting file resolves IDENTICALLY in game and editor. THIS module keeps
//! only the editor host's thin systems: the `Startup` kick-off holding a
//! whole-session persistent folder handle (the GTW-533 editor policy), the
//! own-absence-gated resolve (fail-closed through the shared salvage —
//! ADR-0003), and the ungated live redrive.

use bevy::{
    asset::{AssetEvent, AssetServer, Assets, LoadedFolder, RecursiveDependencyLoadState},
    ecs::system::SystemParam,
    prelude::*,
};
use gdtf_assets::{ContentIntegrityReport, RonAsset, RonAssetAppExt, RonFolderSalvage};
use gdtf_battle_sim::injuries::{InjuryDef, InjuryRegistry, InjuryTables, InjuryWeighting};
use gdtf_content_families::injuries::{
    INJURIES_FOLDER, INJURY_DEF_EXTENSION, INJURY_WEIGHTING_EXTENSION, begin_injuries_salvage,
    build_injury_data, settle_injuries_salvage,
};

/// The editor's PERSISTENT handle to the recursive `content/injuries/` folder
/// load — inserted at `Startup` and NEVER removed (the GTW-533 whole-session
/// persistence policy), so the live redrive can re-enumerate the folder's
/// member handles and holding it keeps every member asset loaded for the
/// file-watcher. A no-bare-types newtype (private inner, [`Deref`] read).
#[derive(Resource, Deref)]
pub(crate) struct InjuriesFolderHandle(Handle<LoadedFolder>);

/// The read-side asset collections + salvage state the injuries resolve /
/// redrive walk, bundled into one `#[derive(SystemParam)]` (the load gate's
/// `GateResources` pattern) so both systems stay under clippy's argument gate.
#[derive(SystemParam)]
pub(crate) struct InjuryAssets<'w> {
    /// The asset server (folder state polls + member-path resolution).
    server:            Res<'w, AssetServer>,
    /// The loaded-folder collection (the member-handle enumeration).
    folders:           Res<'w, Assets<LoadedFolder>>,
    /// The per-injury def collection (`*.injury.ron`).
    defs:              Res<'w, Assets<RonAsset<InjuryDef>>>,
    /// The per-category weighting collection (`*.weighting.ron`).
    weightings:        Res<'w, Assets<RonAsset<InjuryWeighting>>>,
    /// The defs half of the GTW-582 per-file salvage (present only after a
    /// `Failed` folder walk began one).
    def_salvage:       Option<Res<'w, RonFolderSalvage<InjuryDef>>>,
    /// The weightings half of the per-file salvage.
    weighting_salvage: Option<Res<'w, RonFolderSalvage<InjuryWeighting>>>,
}

/// Registers the editor's bespoke injuries `Load` pass onto `app` — the two
/// dedicated-extension RON loaders, the `Startup` kick-off, the gated resolve,
/// and the ungated live redrive.
///
/// SELF-GATES on an [`AssetServer`] being present (the seam ext precedent —
/// registering an asset without one panics): a `MinimalPlugins` harness skips
/// the whole chain and instead seeds the DEFAULT (empty) [`InjuryRegistry`] +
/// [`InjuryTables`] (the GTW-629 headless-fallback rider), so the
/// presence-gated `Load → Editing` transition still releases headless.
pub(crate) fn register_injuries(app: &mut App) {
    if app.world().get_resource::<AssetServer>().is_none() {
        // Headless fallback rider (GTW-629): `init_resource` (not `insert`) so
        // a harness that pre-seeded canonical resources keeps them.
        app.init_resource::<InjuryRegistry>();
        app.init_resource::<InjuryTables>();
        return;
    }
    // The SAME single-owner extension consts the game registers with (GTW-634),
    // so the editor's loaders can never drift from the shipped layout.
    app.init_ron_asset_with_extensions::<InjuryDef>(vec![INJURY_DEF_EXTENSION]);
    app.init_ron_asset_with_extensions::<InjuryWeighting>(vec![INJURY_WEIGHTING_EXTENSION]);
    // The GTW-582 per-file salvage records malformed members here (idempotent —
    // the seam families' registrations init the same resource).
    app.init_resource::<ContentIntegrityReport>();
    app.add_systems(Startup, kick_off_injuries).add_systems(
        Update,
        (
            // Own-absence gating (`bevy-traps.md` #3): the resolve runs only
            // while its OWN registry is absent, so it can never shadow a
            // resolved pair and never starves another branch.
            resolve_injuries.run_if(
                resource_exists::<InjuriesFolderHandle>
                    .and_then(not(resource_exists::<InjuryRegistry>)),
            ),
            // The live redrive is UNGATED (it self-guards on its optional
            // borrows) — hot edits keep rebuilding after `Load` exits because
            // the folder handle persists (GTW-533).
            redrive_injuries,
        ),
    );
}

/// `Startup`: begin the recursive `content/injuries/` folder load and store the
/// persistent [`InjuriesFolderHandle`] (the bespoke twin of the seam's
/// `kick_off_content_family`).
fn kick_off_injuries(mut commands: Commands, server: Res<AssetServer>) {
    commands.insert_resource(InjuriesFolderHandle(server.load_folder(INJURIES_FOLDER)));
}

/// `Update` (handle present, [`InjuryRegistry`] absent): poll the folder load
/// and, once `Loaded`, build BOTH resources through the shared
/// [`build_injury_data`] — the game resolve's exact decision shape:
///
/// - `Failed` → begin the shared per-file salvage (one per asset type); the
///   settle then publishes both resources atomically (or fails closed to the
///   EMPTY pair on an un-enumerable folder — ADR-0003, the no-strand
///   guarantee).
/// - `Loaded` but a member not yet in its collection → retry next frame
///   (never publish a partial pair).
fn resolve_injuries(
    mut commands: Commands,
    handle: Res<InjuriesFolderHandle>,
    assets: InjuryAssets,
    mut report: Option<ResMut<ContentIntegrityReport>>,
) {
    // Salvage-poll path: a prior frame's `Failed` began the per-file salvages —
    // settle BOTH before publishing either resource (each malformed member
    // lands loudly on the shared content-integrity report).
    if let (Some(def_salvage), Some(weighting_salvage)) =
        (&assets.def_salvage, &assets.weighting_salvage)
    {
        settle_injuries_salvage(
            &mut commands,
            &assets.server,
            &assets.defs,
            &assets.weightings,
            def_salvage,
            weighting_salvage,
            report.as_deref_mut(),
        );
        return;
    }

    let folder_state = assets.server.recursive_dependency_load_state(&**handle);
    if matches!(folder_state, RecursiveDependencyLoadState::Failed(_)) {
        begin_injuries_salvage(&mut commands, &assets.server);
        return;
    }
    if matches!(folder_state, RecursiveDependencyLoadState::Loaded)
        && let Some((registry, tables)) = build_injury_data(
            &assets.server,
            &assets.folders,
            &assets.defs,
            &assets.weightings,
            &handle,
        )
    {
        commands.insert_resource(registry);
        commands.insert_resource(tables);
    }
}

/// `Update` (ungated): rebuild BOTH resources on a `Modified` event for ANY
/// member `*.injury.ron` OR `*.weighting.ron` — the editor twin of the game's
/// `redrive_injuries_on_asset_event`, through the SAME shared builder, so a
/// hot edit (or an [`AssetServer::reload`] after a save) yields exactly what a
/// restart would. Self-guards on the optional borrows (`bevy-traps.md` #1),
/// draining both readers so a pre-resolve event never lingers.
fn redrive_injuries(
    mut def_events: MessageReader<AssetEvent<RonAsset<InjuryDef>>>,
    mut weighting_events: MessageReader<AssetEvent<RonAsset<InjuryWeighting>>>,
    handle: Option<Res<InjuriesFolderHandle>>,
    assets: InjuryAssets,
    resources: Option<(ResMut<InjuryRegistry>, ResMut<InjuryTables>)>,
) {
    let (Some(handle), Some((mut registry, mut tables))) = (handle, resources) else {
        // Nothing to rebuild yet — drain so a pre-resolve event does not
        // re-fire once the resources arrive.
        def_events.clear();
        weighting_events.clear();
        return;
    };

    let def_modified = def_events
        .read()
        .any(|event| matches!(event, AssetEvent::Modified { .. }));
    let weighting_modified = weighting_events
        .read()
        .any(|event| matches!(event, AssetEvent::Modified { .. }));
    if !def_modified && !weighting_modified {
        return;
    }

    let Some((rebuilt_registry, rebuilt_tables)) = build_injury_data(
        &assets.server,
        &assets.folders,
        &assets.defs,
        &assets.weightings,
        &handle,
    ) else {
        // A member is mid-reload — keep the current pair; the next event
        // rebuilds.
        return;
    };
    *registry = rebuilt_registry;
    *tables = rebuilt_tables;
    info!(
        "editor injury hot-reload: rebuilt InjuryRegistry + InjuryTables \
         ({} injuries, {} weighting buckets)",
        registry.len(),
        tables.len(),
    );
}
