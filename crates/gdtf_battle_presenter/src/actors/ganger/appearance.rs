//! The ganger-sprite APPEARANCE resolver (GTW-631): ONE pure classifier deciding the
//! atlas index + tint a ganger sprite draws with, and the ONE system stamping both.
//!
//! Before GTW-631 three sites wrote a ganger sprite's `Sprite` appearance channels from
//! diverging derivations: the reframe re-tinted via the stance/aim composition, the
//! life-state system re-tinted via the plain faction/life base, and the hot-reload
//! re-index re-stamped the atlas index while held off the colour channel purely by a
//! "RE-INDEX ONLY" comment convention. Now the decision lives in ONE pure classifier
//! ([`ganger_sprite_appearance`] — the existing faction / downed / stance-aim choice fns
//! COMPOSE inside it, unmerged) and both `Sprite` channels have ONE writer
//! ([`resolve_ganger_appearance`]); the spawn seeds its initial value through the same
//! classifier, so two writers deriving different appearances from the same sim state are
//! unrepresentable rather than conventioned — the same shape as the GTW-627 visibility
//! resolver next door.

use bevy::{ecs::lifecycle::RemovedComponents, prelude::*};
use gdtf_battle_sim::{
    ganger::{Aiming, Facing, Suppressed},
    prelude::{Faction, LifeState, Stance},
};

use super::{
    frame::atlas_index,
    roles::CharacterRoles,
    sprite_map::{GangerSprite, GangerSprites},
    tint::stance_aiming_tint,
};

/// The full drawn appearance of one ganger sprite — which atlas tile it shows and the
/// tint it is modulated by — the ONE classifier's verdict, stamped onto both `Sprite`
/// channels together (GTW-631).
///
/// `atlas_index` is a flat `usize` for [`TextureAtlas::index`](bevy::prelude::TextureAtlas)
/// and the tint is a [`Color`] for `Sprite::color` — both framework plumbing (the
/// no-bare-types carve-out, matching [`atlas_index`]'s return); the CHOICE both encode is
/// [`ganger_sprite_appearance`]'s.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct GangerAppearance {
    /// The drawn atlas tile: the structural `faction_base + facing_frame` sum
    /// ([`atlas_index`]).
    pub(super) atlas_index: usize,
    /// The drawn tint: the faction / life base modulated by stance, aim, and the
    /// suppression drain ([`stance_aiming_tint`]).
    pub(super) tint:        Color,
}

/// The ONE pure ganger-appearance classifier (GTW-631 C1): every input that decides how a
/// ganger sprite LOOKS — faction, facing, stance, aiming, life state, the suppressed
/// flag, and the [`CharacterRoles`] table — composed into the (atlas index, tint) pair.
///
/// The existing choice fns are its INTERNALS, composed not merged: the index is the
/// structural `faction_base + facing_frame` sum ([`atlas_index`], via the 8->4
/// [`facing_frame`](super::facing_frame) map), and the tint is [`stance_aiming_tint`]
/// (the faction / Downed base modulated by stance, aim, and the GTW-526 suppression
/// drain). Each keeps its own single concern; no mode flags.
///
/// The canonical answers for the formerly-divergent combinations, pinned by the GTW-631
/// C4 unit matrix:
///
/// * **stance / aim + Downed** — the Downed grey-out WINS: an out-of-fight body is
///   neither dimmed further by Prone nor brightened by aim.
/// * **suppressed + Downed** — the Downed grey-out WINS: the suppression desaturation
///   applies only to a live ganger.
/// * **Dead** — classified as the live faction tint for completeness (a Dead ganger's
///   sprite is despawned, or a deferred corpse the writer never re-styles — see
///   [`resolve_ganger_appearance`]).
#[must_use]
pub(super) fn ganger_sprite_appearance(
    faction: Faction,
    facing: Facing,
    stance: Stance,
    aiming: Aiming,
    life: LifeState,
    suppressed: bool,
    roles: &CharacterRoles,
) -> GangerAppearance {
    GangerAppearance {
        atlas_index: atlas_index(roles, faction, facing),
        tint:        stance_aiming_tint(faction, life, stance, aiming, suppressed),
    }
}

/// The read-only ganger fields the appearance stamp reads — the [`QueryData`] tuple,
/// factored out to keep the [`resolve_ganger_appearance`] queries under the
/// `type_complexity` clippy gate.
///
/// [`QueryData`]: bevy::ecs::query::QueryData
type AppearanceData = (
    Entity,
    &'static Faction,
    &'static Facing,
    &'static Stance,
    &'static Aiming,
    &'static LifeState,
    Option<&'static Suppressed>,
);

/// The "any of facing / stance / aiming / suppressed changed" [`QueryFilter`] — the
/// pre-GTW-631 reframe union, kept as its own named term inside [`AppearanceChanged`].
///
/// [`Changed<Suppressed>`](Suppressed) catches suppression being APPLIED (the sim inserts
/// the component — GTW-526 C2); a suppressed ganger's auto-stance drop (C5) ALSO trips
/// [`Changed<Stance>`], so the tint is doubly guaranteed to refresh on application.
/// Suppression being CLEARED is a component REMOVAL (the sim `remove`s it — C6), which
/// `Changed` does NOT observe, so [`resolve_ganger_appearance`] additionally drains
/// [`RemovedComponents<Suppressed>`](RemovedComponents) to un-tint a no-longer-suppressed
/// ganger.
///
/// [`QueryFilter`]: bevy::ecs::query::QueryFilter
type ReframeChanged = Or<(
    Changed<Facing>,
    Changed<Stance>,
    Changed<Aiming>,
    Changed<Suppressed>,
)>;

/// The full "this ganger's LOOK may have changed" trigger (GTW-631 C2): the
/// [`ReframeChanged`] union plus [`Changed<LifeState>`](LifeState) — the Downed grey-out
/// / revive re-tint the life-state system used to own now rides the same writer.
type AppearanceChanged = Or<(ReframeChanged, Changed<LifeState>)>;

/// `Update` ([`PresenterSystems::Scene`](crate::PresenterSystems), GTW-631 C2): the ONE
/// writer of every ganger sprite's drawn appearance — atlas index AND tint, stamped
/// together from the one pure classifier (`ganger_sprite_appearance`).
///
/// Three drive sources, one derivation:
///
/// * **The changed-state query** (`AppearanceChanged`: facing / stance / aiming /
///   suppressed / life state) — recompute the changed ganger's appearance in place.
/// * **[`RemovedComponents<Suppressed>`](RemovedComponents)** — suppression being CLEARED
///   is a component removal, which `Changed` does not observe; each just-cleared ganger
///   is re-read through the full-set query (now carrying no [`Suppressed`], so the
///   classifier sees `suppressed = false`) and returns to its ordinary tint.
/// * **A [`CharacterRoles`] change** — the GTW-375 hot-reload (the GTW-564 hot-RON
///   redrive) overwrites the resident table, moving each faction's whole 4-frame actor
///   run to a new base, so EVERY mapped ganger is re-stamped from the fresh table (the
///   dissolved dedicated re-index system, GTW-631 C3 — now stamping BOTH channels, so a
///   stale tint can never survive a table reload). Expressed as `roles.is_changed()`
///   INSIDE the system rather than a `resource_changed` run condition: as a run
///   condition it would gate the OTHER two drivers off on every roles-quiet frame; the
///   change-tick comparison (against this system's last run) is identical. The one-time
///   initial resolve also reads as changed — harmless, it re-stamps the just-seeded
///   values.
///
/// A [`Dead`](LifeState::Dead) ganger is never re-styled: death presentation is the
/// despawn discrimination's ([`update_ganger_life_state`](super::death::update_ganger_life_state),
/// GTW-331) — its sprite is either already despawned (the lookup misses) or a deferred
/// corpse awaiting its killing tracer, whose last-drawn look is preserved until the
/// impact despawn. A just-spawned ganger's sprite may not have materialized yet (the
/// deferred `spawn_scene`, GTW-322) — it is skipped, and needs no later heal because the
/// spawn seeded the SAME classifier's verdict.
///
/// Param-only (`bevy-traps.md` #7): [`Res<GangerSprites>`], [`Res<CharacterRoles>`], the
/// changed-state ganger query, the full-set ganger query (the removal / roles-change
/// re-read), the [`RemovedComponents<Suppressed>`](RemovedComponents) drain, and the ONE
/// `Query<&mut Sprite, With<GangerSprite>>` in the crate (GTW-631 A1).
pub fn resolve_ganger_appearance(
    sprites: Res<GangerSprites>,
    roles: Res<CharacterRoles>,
    changed: Query<AppearanceData, AppearanceChanged>,
    all: Query<AppearanceData>,
    mut removed: RemovedComponents<Suppressed>,
    mut presenters: Query<&mut Sprite, With<GangerSprite>>,
) {
    // The roles-change path: a hot-reloaded table moves every faction base, so re-stamp
    // EVERY mapped ganger from the fresh table. The full set supersedes the changed and
    // removal subsets this frame (a removal re-read through `all` already sees the
    // component gone), so the pending removals are drained and dropped.
    if roles.is_changed() {
        removed.clear();
        for data in &all {
            stamp_appearance(&sprites, &roles, &mut presenters, data);
        }
        return;
    }
    // The changed-state path: every ganger whose appearance-deciding sim state Changed.
    for data in &changed {
        stamp_appearance(&sprites, &roles, &mut presenters, data);
    }
    // The suppression-CLEARED path: a removal is not a `Changed`, so drain the removals
    // and re-stamp each just-cleared ganger from its full-set data. A ganger whose
    // entity despawned is skipped by the `all.get` miss.
    for entity in removed.read() {
        if let Ok(data) = all.get(entity) {
            stamp_appearance(&sprites, &roles, &mut presenters, data);
        }
    }
}

/// Stamp one ganger's classifier verdict onto its presenter sprite — BOTH channels, the
/// shared body of every [`resolve_ganger_appearance`] path.
///
/// Looks the presenter sprite up through [`GangerSprites`] and writes the
/// [`ganger_sprite_appearance`] verdict: the facing-correct atlas index AND the composed
/// tint. A [`Dead`](LifeState::Dead) ganger, an unmapped ganger, and a
/// not-yet-materialized sprite are all skipped (see the system doc).
fn stamp_appearance(
    sprites: &GangerSprites,
    roles: &CharacterRoles,
    presenters: &mut Query<&mut Sprite, With<GangerSprite>>,
    data: (
        Entity,
        &Faction,
        &Facing,
        &Stance,
        &Aiming,
        &LifeState,
        Option<&Suppressed>,
    ),
) {
    let (entity, faction, facing, stance, aiming, life, suppressed) = data;
    // Death presentation is the despawn discriminator's (GTW-331), never a re-style: a
    // deferred corpse keeps its last-drawn look until its killing tracer lands.
    if matches!(life, LifeState::Dead) {
        return;
    }
    let Some(presenter) = sprites.sprite_for(entity) else {
        return;
    };
    let Ok(mut sprite) = presenters.get_mut(presenter) else {
        return;
    };
    let appearance = ganger_sprite_appearance(
        *faction,
        *facing,
        *stance,
        *aiming,
        *life,
        suppressed.is_some(),
        roles,
    );
    if let Some(atlas) = sprite.texture_atlas.as_mut() {
        atlas.index = appearance.atlas_index;
    }
    sprite.color = appearance.tint;
}
