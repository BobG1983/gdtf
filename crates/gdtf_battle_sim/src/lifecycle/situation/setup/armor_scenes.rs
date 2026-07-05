//! [`worn_piece_scenes`] — `bsn!` composition of the six worn-armor-piece related
//! entities (GTW-323 slice 1, ADR-0004), spawn-and-related via
//! [`Wears`](crate::armor::Wears) by [`setup_battle`](super::setup_battle).

use bevy::scene::{Scene, SceneList, bsn, bsn_list, template_value};

use crate::armor::{
    ArmorFloor, ArmorHardness, ArmorIntegrity, ArmorProtection, ArmorSpec, BodyPart,
};

/// Compose the six worn-armor-piece **related scenes** for a ganger as a
/// [`SceneList`] — one piece entity per [`BodyPart`], each carrying its stat
/// components, to spawn-and-relate via `Wears` (GTW-323 slice 1, ADR-0004).
///
/// The [`setup_battle`](super::setup_battle) spawn loop hands this list to
/// [`queue_spawn_related_scenes::<Wears>`](bevy::scene::EntityCommandsSceneExt::queue_spawn_related_scenes)
/// on the freshly-spawned ganger entity: the framework spawns one entity per scene,
/// applies the scene's components, and inserts [`WornBy`](crate::armor::WornBy)`(ganger)` on each — whose
/// back-reference hook populates the ganger's [`Wears`](crate::armor::Wears) collection automatically.
///
/// Each piece scene tags the entity with its [`BodyPart`] (so the `struck_piece`
/// lookup keys `ganger → Wears → the BodyPart-tagged piece`) and carries the five
/// per-piece stat components ([`ArmorFloor`] / [`ArmorProtection`] / [`ArmorIntegrity`]
/// / [`ArmorHardness`] / `ArmorType`) read **by value** from the resolved
/// [`ArmorSpec`] in [`BodyPart::ALL`] order. These piece entities are the ONLY armor
/// storage — GTW-323 slice 3 removed the transient ganger-side copy, so no armor stat
/// data is stored on the ganger. The four stat newtypes inline via their
/// `Type::new(value)` `bsn!` form; the
/// runtime-valued [`BodyPart`] tag and `ArmorType` (fieldless enums with no `new`
/// grammar form) bridge via [`template_value`] (their GTW-322 spawn-seed-sentinel
/// [`Default`]s seed the slot before the authored value overwrites it), tuple-composed
/// onto the same piece entity.
pub(super) fn worn_piece_scenes(spec: &ArmorSpec) -> impl SceneList {
    // bsn! `Type::new(expr)` stores a DEFERRED constructor, so every captured value
    // must be OWNED (the GTW-322 `'static` finding). Read each piece by value out of
    // the spec FIRST (ArmorPiece is Copy), then let the macro capture the owned locals.
    let pieces = spec.pieces();
    bsn_list! {
        worn_piece_scene(BodyPart::Head, pieces[BodyPart::Head.index()]),
        worn_piece_scene(BodyPart::Torso, pieces[BodyPart::Torso.index()]),
        worn_piece_scene(BodyPart::LeftArm, pieces[BodyPart::LeftArm.index()]),
        worn_piece_scene(BodyPart::RightArm, pieces[BodyPart::RightArm.index()]),
        worn_piece_scene(BodyPart::LeftLeg, pieces[BodyPart::LeftLeg.index()]),
        worn_piece_scene(BodyPart::RightLeg, pieces[BodyPart::RightLeg.index()]),
    }
}

/// Compose ONE worn-armor-piece entity as a `bsn!` [`Scene`] — its [`BodyPart`] tag
/// plus the five stat components, read by value from `piece` (GTW-323 slice 1).
///
/// The four stat newtypes inline via `Type::new(value)`; the runtime-valued
/// [`BodyPart`] tag and `ArmorType` fieldless enums bridge via [`template_value`]
/// (no `bsn!` grammar form), tuple-composed onto the same piece entity (the GTW-322
/// runtime-value path). The [`WornBy`](crate::armor::WornBy) back-reference is inserted by the framework's
/// `queue_spawn_related_scenes::<Wears>` wiring, NOT here, so it is absent from this
/// scene.
fn worn_piece_scene(part: BodyPart, piece: crate::armor::ArmorPiece) -> impl Scene {
    // Bind every inline value to an owned local FIRST (the bsn! `'static` finding).
    let floor = *piece.floor;
    let protection = *piece.protection;
    let integrity = *piece.integrity;
    let hardness = *piece.hardness;
    let armor_type = piece.armor_type;
    (
        bsn! {
            ArmorFloor::new(floor)
            ArmorProtection::new(protection)
            ArmorIntegrity::new(integrity)
            ArmorHardness::new(hardness)
        },
        // The runtime-valued fieldless enums with no `bsn!` grammar form, bridged via
        // `template_value` and tuple-composed onto the SAME piece entity (GTW-322).
        template_value(part),
        template_value(armor_type),
    )
}
