use bevy::prelude::*;
use gdtf_battle_presenter::{ContextDepth, IsolateView, ViewMode};
use gdtf_content_editor::{
    ArmorDraft, AttachmentDraft, CanvasZoom, CurrentEditLevel, EditorMap, EditorMode, FieldDraft,
    GangDraft, HoveredCell, InjuryDraft, InjurySubTab, MapEditorSession, MeleeWeaponDraft,
    PreviewPan, SpriteDraft, TerrainDraft, ThemeDraft, WeaponDraft, WeightingDraft,
};

pub(crate) fn assert_all_scoped_resources_absent(app: &App, when: &str) {
    let world = app.world();
    assert!(
        world.get_resource::<EditorMode>().is_none(),
        "EditorMode {when}"
    );
    assert!(
        world.get_resource::<MapEditorSession>().is_none(),
        "MapEditorSession {when}",
    );
    assert!(
        world.get_resource::<EditorMap>().is_none(),
        "EditorMap {when}"
    );
    assert!(
        world.get_resource::<CurrentEditLevel>().is_none(),
        "CurrentEditLevel {when}",
    );
    assert!(
        world.get_resource::<CanvasZoom>().is_none(),
        "CanvasZoom {when}"
    );
    assert!(
        world.get_resource::<TerrainDraft>().is_none(),
        "TerrainDraft {when}"
    );
    assert!(
        world.get_resource::<ThemeDraft>().is_none(),
        "ThemeDraft {when}"
    );
    assert!(
        world.get_resource::<HoveredCell>().is_none(),
        "HoveredCell {when}"
    );
    assert!(
        world.get_resource::<PreviewPan>().is_none(),
        "PreviewPan {when}"
    );
    assert!(
        world.get_resource::<ViewMode>().is_none(),
        "ViewMode {when}"
    );
    assert!(
        world.get_resource::<IsolateView>().is_none(),
        "IsolateView {when}"
    );
    assert!(
        world.get_resource::<GangDraft>().is_none(),
        "GangDraft {when}"
    );
    assert!(
        world.get_resource::<ArmorDraft>().is_none(),
        "ArmorDraft {when}"
    );
    assert!(
        world.get_resource::<InjuryDraft>().is_none(),
        "InjuryDraft {when}"
    );
    assert!(
        world.get_resource::<WeightingDraft>().is_none(),
        "WeightingDraft {when}"
    );
    assert!(
        world.get_resource::<InjurySubTab>().is_none(),
        "InjurySubTab {when}"
    );
    assert!(
        world.get_resource::<SpriteDraft>().is_none(),
        "SpriteDraft {when}"
    );
    assert!(
        world.get_resource::<AttachmentDraft>().is_none(),
        "AttachmentDraft {when}"
    );
    assert!(
        world.get_resource::<WeaponDraft>().is_none(),
        "WeaponDraft {when}"
    );
    assert!(
        world.get_resource::<MeleeWeaponDraft>().is_none(),
        "MeleeWeaponDraft {when}"
    );
    assert!(
        world.get_resource::<FieldDraft>().is_none(),
        "FieldDraft {when}"
    );
}

pub(crate) fn assert_all_scoped_resources_seeded(app: &App) {
    let world = app.world();
    assert_eq!(
        world.get_resource::<EditorMode>(),
        Some(&EditorMode::default()),
        "EditorMode seeds to the default Prefab mode ",
    );
    assert_eq!(
        world.get_resource::<EditorMap>(),
        Some(&EditorMap::new()),
        "EditorMap seeds empty ",
    );
    assert_eq!(
        world.get_resource::<CurrentEditLevel>(),
        Some(&CurrentEditLevel::ground()),
        "CurrentEditLevel seeds to the ground storey ",
    );
    assert_eq!(
        world.get_resource::<CanvasZoom>(),
        Some(&CanvasZoom::identity()),
        "CanvasZoom seeds to the unzoomed identity ",
    );
    assert_eq!(
        world.get_resource::<TerrainDraft>(),
        Some(&TerrainDraft::default()),
        "TerrainDraft seeds to a fresh default draft ",
    );
    assert_eq!(
        world.get_resource::<HoveredCell>(),
        Some(&HoveredCell::new()),
        "HoveredCell seeds empty — nothing hovered (C1.5)",
    );
    assert_eq!(
        world.get_resource::<PreviewPan>(),
        Some(&PreviewPan::origin()),
        "PreviewPan seeds to the origin (C4.8)",
    );
    assert_eq!(
        world.get_resource::<ViewMode>(),
        Some(&ViewMode::default()),
        "ViewMode seeds to the default DownToActive ",
    );
    assert_eq!(
        world.get_resource::<IsolateView>(),
        Some(&IsolateView::On(ContextDepth::new(1))),
        "IsolateView seeds ON with one onion storey below — the editor default \
         (the battlescape's own init_resource default stays Off)",
    );
    assert_eq!(
        world.get_resource::<GangDraft>(),
        Some(&GangDraft::default()),
        "GangDraft seeds to the pristine autoload-pending form ",
    );
    assert_eq!(
        world.get_resource::<ArmorDraft>(),
        Some(&ArmorDraft::default()),
        "ArmorDraft seeds to the pristine autoload-pending form ",
    );
    assert_eq!(
        world.get_resource::<InjuryDraft>(),
        Some(&InjuryDraft::default()),
        "InjuryDraft seeds to the pristine autoload-pending form ",
    );
    assert_eq!(
        world.get_resource::<WeightingDraft>(),
        Some(&WeightingDraft::default()),
        "WeightingDraft seeds to the pristine autoload-pending form ",
    );
    assert_eq!(
        world.get_resource::<InjurySubTab>(),
        Some(&InjurySubTab::default()),
        "InjurySubTab seeds to Def, so a fresh authoring session opens the def form ",
    );
    assert_eq!(
        world.get_resource::<SpriteDraft>(),
        Some(&SpriteDraft::default()),
        "SpriteDraft seeds to the pristine autoload-pending form ",
    );
    assert_eq!(
        world.get_resource::<AttachmentDraft>(),
        Some(&AttachmentDraft::default()),
        "AttachmentDraft seeds to the pristine autoload-pending form ",
    );
    assert_eq!(
        world.get_resource::<WeaponDraft>(),
        Some(&WeaponDraft::default()),
        "WeaponDraft seeds to the pristine autoload-pending form ",
    );
    assert_eq!(
        world.get_resource::<MeleeWeaponDraft>(),
        Some(&MeleeWeaponDraft::default()),
        "MeleeWeaponDraft seeds to the pristine autoload-pending form ",
    );
    assert_eq!(
        world.get_resource::<FieldDraft>(),
        Some(&FieldDraft::default()),
        "FieldDraft seeds to the pristine autoload-pending form ",
    );
    assert_minted_seeds(world);
}

fn assert_minted_seeds(world: &World) {
    let theme_draft = world.get_resource::<ThemeDraft>();
    assert!(
        theme_draft.is_some(),
        "ThemeDraft must be present in Editing"
    );
    let Some(theme_draft) = theme_draft else {
        return;
    };
    assert_eq!(
        theme_draft.display_name(),
        "",
        "ThemeDraft seeds with an empty display name (a fresh NEW-theme draft)",
    );
    assert!(
        theme_draft.terrain().is_empty(),
        "ThemeDraft seeds with an empty terrain palette",
    );
    assert_eq!(
        theme_draft.default_floor(),
        None,
        "ThemeDraft seeds with no default floor chosen",
    );
    let session = world.get_resource::<MapEditorSession>();
    assert!(
        session.is_some(),
        "MapEditorSession must be present in Editing"
    );
    let Some(session) = session else {
        return;
    };
    assert_eq!(
        session.grid_size(),
        MapEditorSession::default().grid_size(),
        "MapEditorSession seeds with the full default grid",
    );
    assert_eq!(
        session.selected_tile(),
        None,
        "MapEditorSession seeds with no paint tile selected",
    );
}
