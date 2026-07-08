//! The roster ASSERT helpers of the `state_scoped_resources` suite — one absence /
//! seeded block per `Editing`-scoped model resource (these grow with every Workbench
//! mode, which is why they live apart from the harness — the GTW-670 dir-form split).

use bevy::prelude::*;
use gdtf_battle_presenter::{ContextDepth, IsolateView, ViewMode};
use gdtf_content_editor::{
    ArmorDraft, AttachmentDraft, CanvasZoom, CurrentEditLevel, EditorMap, EditorMode, GangDraft,
    HoveredCell, InjuryDraft, MapEditorSession, MeleeWeaponDraft, PreviewPan, SpriteDraft,
    TerrainDraft, ThemeDraft, WeaponDraft, WeightingDraft,
};

/// Asserts every one of the nineteen `Editing`-scoped model resources is absent.
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
}

/// Asserts every scoped resource is present with the seed its
/// `init_state_scoped_resource` registration captured — the IDENTICAL
/// constructors the hand-stamped `editor_resources.rs` inserts used.
pub(crate) fn assert_all_scoped_resources_seeded(app: &App) {
    let world = app.world();
    assert_eq!(
        world.get_resource::<EditorMode>(),
        Some(&EditorMode::default()),
        "EditorMode seeds to the default Prefab mode (GTW-474)",
    );
    assert_eq!(
        world.get_resource::<EditorMap>(),
        Some(&EditorMap::new()),
        "EditorMap seeds empty (GTW-426)",
    );
    assert_eq!(
        world.get_resource::<CurrentEditLevel>(),
        Some(&CurrentEditLevel::ground()),
        "CurrentEditLevel seeds to the ground storey (GTW-500 C1)",
    );
    assert_eq!(
        world.get_resource::<CanvasZoom>(),
        Some(&CanvasZoom::identity()),
        "CanvasZoom seeds to the unzoomed identity (GTW-500 C3)",
    );
    assert_eq!(
        world.get_resource::<TerrainDraft>(),
        Some(&TerrainDraft::default()),
        "TerrainDraft seeds to a fresh default draft (GTW-474)",
    );
    assert_eq!(
        world.get_resource::<HoveredCell>(),
        Some(&HoveredCell::new()),
        "HoveredCell seeds empty — nothing hovered (GTW-512 C1.5)",
    );
    assert_eq!(
        world.get_resource::<PreviewPan>(),
        Some(&PreviewPan::origin()),
        "PreviewPan seeds to the origin (GTW-515 C4.8)",
    );
    assert_eq!(
        world.get_resource::<ViewMode>(),
        Some(&ViewMode::default()),
        "ViewMode seeds to the default DownToActive (GTW-532)",
    );
    assert_eq!(
        world.get_resource::<IsolateView>(),
        Some(&IsolateView::On(ContextDepth::new(1))),
        "IsolateView seeds ON with one onion storey below — the GTW-594 editor default \
         (the battlescape's own init_resource default stays Off)",
    );
    assert_eq!(
        world.get_resource::<GangDraft>(),
        Some(&GangDraft::default()),
        "GangDraft seeds to the pristine autoload-pending form (GTW-636)",
    );
    assert_eq!(
        world.get_resource::<ArmorDraft>(),
        Some(&ArmorDraft::default()),
        "ArmorDraft seeds to the pristine autoload-pending form (GTW-479)",
    );
    assert_eq!(
        world.get_resource::<InjuryDraft>(),
        Some(&InjuryDraft::default()),
        "InjuryDraft seeds to the pristine autoload-pending form (GTW-654)",
    );
    assert_eq!(
        world.get_resource::<WeightingDraft>(),
        Some(&WeightingDraft::default()),
        "WeightingDraft seeds to the pristine autoload-pending form (GTW-654 C2)",
    );
    assert_eq!(
        world.get_resource::<SpriteDraft>(),
        Some(&SpriteDraft::default()),
        "SpriteDraft seeds to the pristine autoload-pending form (GTW-664)",
    );
    assert_eq!(
        world.get_resource::<AttachmentDraft>(),
        Some(&AttachmentDraft::default()),
        "AttachmentDraft seeds to the pristine autoload-pending form (GTW-669)",
    );
    assert_eq!(
        world.get_resource::<WeaponDraft>(),
        Some(&WeaponDraft::default()),
        "WeaponDraft seeds to the pristine autoload-pending form (GTW-670)",
    );
    assert_eq!(
        world.get_resource::<MeleeWeaponDraft>(),
        Some(&MeleeWeaponDraft::default()),
        "MeleeWeaponDraft seeds to the pristine autoload-pending form (GTW-671)",
    );
    assert_minted_seeds(world);
}

/// The two seeds whole-value equality cannot pin, asserted field by field —
/// split out of [`assert_all_scoped_resources_seeded`] purely for the
/// `too_many_lines` band as the fifteenth resource joined (GTW-654; the
/// sixteenth through nineteenth — GTW-664's `SpriteDraft`, GTW-669's
/// `AttachmentDraft`, GTW-670's `WeaponDraft`, and GTW-671's
/// `MeleeWeaponDraft` — pin by whole-value equality above).
fn assert_minted_seeds(world: &World) {
    // ThemeDraft's seed (`ThemeDraft::default` -> `new_theme`) MINTS a fresh
    // `ThemeUuid` per entry by design (GTW-475 C4), so whole-value equality
    // against another fresh default would fail on the key; assert the seeded
    // form state field by field instead.
    // `let … else` keeps the test panic-free per the workspace lints (the
    // preceding assert is what fails the test on absence).
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
    // MapEditorSession seeds to `MapEditorSession::default()` (nil theme, no
    // floor, the full 60×60×8 grid, no paint tile). The pre-existing
    // `seed_default_theme` drive (GTW-421) may already have re-seeded the
    // theme/floor PAIR from the resolved registry by the time we read — that
    // is unchanged production behavior, so assert the drive-untouched seed
    // fields exactly.
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
