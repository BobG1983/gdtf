//! GTW-574 — the kind-identity pins tying the editor pick list to the canonical
//! [`TerrainPieceKind`] discriminant: the round-trip identity (AC1), the
//! `SEGMENT_ORDER` completeness pin (AC2), the Emplacement projection carrying the
//! exact mounted-weapon key (AC3, unit half), and the fail-closed
//! missing-mounted-weapon rule (AC4, unit half).

use gdtf_battle_presenter::TileRole;
use gdtf_battle_sim::{
    terrain::{
        def::{TerrainDef, TerrainDefRegistry, TerrainPresenterKind, TerrainSimKind},
        entity::TerrainPieceKind,
    },
    weapon::WeaponName,
};

use super::support::key;
use crate::terrain_form::{
    SaveTerrainError, TerrainDraft, TerrainKindChoice, draft_to_terrain_def, serialize_terrain_def,
};

/// A draft of the given pick-list kind, populated enough to project: named, and (for
/// Emplacement) carrying a selected mounted weapon so the fail-closed gate passes.
fn draft_of(choice: TerrainKindChoice) -> TerrainDraft {
    let mut draft = TerrainDraft::default();
    draft.set_display_name("GTW574 Kind Probe".to_owned());
    draft.set_kind(choice);
    if choice == TerrainKindChoice::Emplacement {
        draft.set_mounted_weapon(Some(WeaponName::new("heavy_stubber".to_owned())));
    }
    draft
}

/// AC1 — the canonical round-trip is kind-level IDENTITY for EVERY [`TerrainPieceKind`]
/// variant: `From<TerrainPieceKind>` into the pick list, composed with the choice →
/// sim-kind projection (`draft_to_terrain_def`), lands back on the SAME canonical kind
/// via [`TerrainSimKind::kind`] (and the presenter half agrees via
/// [`TerrainPresenterKind::kind`]). A new terrain kind that reaches the pick list but
/// projects to the wrong sim variant fails here.
#[test]
fn piece_kind_round_trips_through_choice_and_projection() {
    for kind in TerrainPieceKind::ALL {
        let choice = TerrainKindChoice::from(kind);
        let draft = draft_of(choice);
        let Ok(def) = draft_to_terrain_def(&draft, key()) else {
            unreachable!("a populated {kind:?} draft must project (the fixture selects a weapon)")
        };
        assert_eq!(
            def.sim_kind.kind(),
            kind,
            "From<TerrainPieceKind> ∘ draft_to_terrain_def must be kind-level identity for \
             {kind:?} (GTW-574 AC1)",
        );
        assert_eq!(
            def.presenter_kind.kind(),
            kind,
            "the projected presenter kind must agree with the canonical kind for {kind:?} \
             (GTW-574 AC1)",
        );
    }
}

/// AC2 — the pick list can never silently drop a kind again: `SEGMENT_ORDER` OFFERS
/// every [`TerrainPieceKind`] variant (via the exhaustive `From` tie), and offers
/// nothing twice. This is the pin that would have caught the pre-GTW-574 drift where
/// `Emplacement` (a live sim kind since GTW-543) never reached the editor.
#[test]
fn segment_order_covers_every_piece_kind() {
    for kind in TerrainPieceKind::ALL {
        let choice = TerrainKindChoice::from(kind);
        assert!(
            TerrainKindChoice::SEGMENT_ORDER.contains(&choice),
            "SEGMENT_ORDER must offer {kind:?} (its pick-list image {choice:?}) — the editor \
             pick list may never silently drop a canonical kind (GTW-574 AC2)",
        );
    }
    assert_eq!(
        TerrainKindChoice::SEGMENT_ORDER.len(),
        TerrainPieceKind::ALL.len(),
        "SEGMENT_ORDER must offer each canonical kind exactly once (GTW-574 AC2)",
    );
}

/// AC3 (unit half) — an Emplacement draft with a selected [`WeaponName`] projects into
/// the Emplacement sim + presenter kinds carrying that EXACT key, serializes, and
/// round-trips through the GTW-487 loader schema into a [`TerrainDefRegistry`]. (The
/// live-app half — the real editor draft + the shipped `WeaponRegistry` + the real fs
/// write — lives in `tests/terrain_mode.rs`.)
#[test]
fn emplacement_draft_projects_serializes_and_registers() {
    let weapon = WeaponName::new("heavy_stubber".to_owned());
    let mut draft = TerrainDraft::default();
    draft.set_display_name("Heavy Stubber Nest".to_owned());
    draft.set_kind(TerrainKindChoice::Emplacement);
    draft.set_graphic(TileRole::Emplacement);
    draft.set_mounted_weapon(Some(weapon.clone()));

    let Ok(def) = draft_to_terrain_def(&draft, key()) else {
        unreachable!("an Emplacement draft with a selected weapon must project (C6)")
    };
    assert!(
        matches!(
            &def.sim_kind,
            TerrainSimKind::Emplacement { mounted_weapon, .. } if *mounted_weapon == weapon
        ),
        "the projected sim kind must be Emplacement carrying the EXACT selected weapon key",
    );
    assert!(
        matches!(
            &def.presenter_kind,
            TerrainPresenterKind::Emplacement { graphic_name }
                if &***graphic_name == TileRole::Emplacement.as_key()
        ),
        "the projected presenter kind must be Emplacement carrying the chosen graphic role",
    );

    // Serialize + parse back through the GTW-487 loader's schema, then resolve through a
    // TerrainDefRegistry exactly as the Load pass would.
    let Ok(text) = serialize_terrain_def(&def) else {
        unreachable!("an Emplacement def must serialize")
    };
    let Ok(reloaded) = ron::de::from_str::<TerrainDef>(&text) else {
        unreachable!("the serialized Emplacement def must parse back (the loader's schema)")
    };
    assert_eq!(
        reloaded, def,
        "the Emplacement def must survive the RON round-trip field-for-field",
    );
    let registry = TerrainDefRegistry::new([(key(), reloaded)]);
    assert!(
        matches!(
            registry.def(&key()).map(|d| &d.sim_kind),
            Some(TerrainSimKind::Emplacement { mounted_weapon, .. }) if *mounted_weapon == weapon
        ),
        "the registry must resolve the Emplacement def with its mounted-weapon key intact (AC3)",
    );
}

/// AC4 (unit half) — FAIL-CLOSED: an Emplacement draft with NO selected mounted weapon
/// projects to the typed [`SaveTerrainError::MissingMountedWeapon`] — no panic, no
/// silent default weapon, no def produced. (The nothing-written fs half lives in
/// `tests/terrain_mode.rs`.)
#[test]
fn emplacement_without_weapon_fails_closed() {
    let mut draft = TerrainDraft::default();
    draft.set_display_name("Unarmed Nest".to_owned());
    draft.set_kind(TerrainKindChoice::Emplacement);
    assert_eq!(
        draft.mounted_weapon(),
        None,
        "a fresh Emplacement draft has no mounted weapon selected",
    );

    assert_eq!(
        draft_to_terrain_def(&draft, key()).err(),
        Some(SaveTerrainError::MissingMountedWeapon),
        "projecting an Emplacement draft without a weapon must return the typed \
         MissingMountedWeapon error (GTW-574 C6 — fail-closed)",
    );
}

/// GTW-574 C5 — the mounted-weapon selection is Emplacement-gated fail-closed, the C2
/// footfall precedent: a commit on a non-Emplacement kind is IGNORED, and switching the
/// kind away from Emplacement CLEARS a previous selection.
#[test]
fn mounted_weapon_gate_holds_fail_closed() {
    let weapon = WeaponName::new("heavy_stubber".to_owned());

    // A commit on a non-Emplacement kind is ignored.
    let mut draft = TerrainDraft::default();
    draft.set_kind(TerrainKindChoice::Wall);
    draft.set_mounted_weapon(Some(weapon.clone()));
    assert_eq!(
        draft.mounted_weapon(),
        None,
        "a mounted-weapon commit on a non-Emplacement kind is ignored (C5 fail-closed)",
    );

    // An Emplacement draft keeps the selection…
    draft.set_kind(TerrainKindChoice::Emplacement);
    draft.set_mounted_weapon(Some(weapon.clone()));
    assert_eq!(
        draft.mounted_weapon(),
        Some(&weapon),
        "an Emplacement kind keeps the selected mounted weapon (C5)",
    );

    // …and switching away clears it (no stale gun survives a kind change).
    draft.set_kind(TerrainKindChoice::Slab);
    assert_eq!(
        draft.mounted_weapon(),
        None,
        "switching off Emplacement clears the mounted weapon (C5 fail-closed)",
    );
}
