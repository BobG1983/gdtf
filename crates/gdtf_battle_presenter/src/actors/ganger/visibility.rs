//! Ganger sprite visibility from storey band and squad fog.

use bevy::{ecs::system::SystemParam, prelude::*};
use gdtf_battle_sim::{
    battle::PlayerFaction,
    prelude::{CellLevel, Faction, LifeState, Position},
    visibility::{FactionRelation, SquadVisibility, is_ganger_visible},
};

use super::sprite_map::{GangerSprite, GangerSprites};
use crate::{
    ActiveLevel, IsolateView, StoreyViewMode, ViewMode, actors::quiet::set_visibility_quiet,
    playback::DrawnLife,
};

pub(super) struct GangerFogFacts<'a> {
    squad: &'a SquadVisibility,
    player: Option<PlayerFaction>,
}

impl<'a> GangerFogFacts<'a> {
    pub(super) const fn new(squad: &'a SquadVisibility, player: Option<PlayerFaction>) -> Self {
        Self { squad, player }
    }
}

pub(super) fn actor_relation(
    player: Option<PlayerFaction>,
    faction: Faction,
    life: LifeState,
) -> FactionRelation {
    let is_player = player.is_some_and(|p| *p == faction);
    if is_player && *life.is_active() {
        FactionRelation::OwnSquad
    } else {
        FactionRelation::Other
    }
}

pub(super) fn classify_ganger_visibility(
    pos: &Position,
    faction: Faction,
    life: LifeState,
    active: ActiveLevel,
    mode: StoreyViewMode,
    fog: Option<&GangerFogFacts<'_>>,
) -> Visibility {
    let in_drawn_band = active.draws_storey(pos.level(), mode);
    let shown_by_fog = fog.is_none_or(|facts| {
        let key: CellLevel = **pos;
        *is_ganger_visible(
            facts.squad,
            &key,
            actor_relation(facts.player, faction, life),
        )
    });
    if in_drawn_band && shown_by_fog {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    }
}

/// Bundled storey and fog facts for classifying ganger visibility.
#[derive(SystemParam)]
pub struct GangerVisibilityFacts<'w> {
    active: Res<'w, ActiveLevel>,
    view: Res<'w, ViewMode>,
    isolate: Res<'w, IsolateView>,
    squad: Option<Res<'w, SquadVisibility>>,
    player: Option<Res<'w, PlayerFaction>>,
}

impl GangerVisibilityFacts<'_> {
    pub(super) fn classify(&self, pos: &Position, faction: Faction, life: LifeState) -> Visibility {
        let fog = self
            .squad
            .as_deref()
            .map(|squad| GangerFogFacts::new(squad, self.player.as_deref().copied()));
        classify_ganger_visibility(
            pos,
            faction,
            life,
            *self.active,
            StoreyViewMode::new(*self.view, *self.isolate),
            fog.as_ref(),
        )
    }
}

/// Update every ganger sprite's visibility from storey band and fog.
pub fn resolve_ganger_visibility(
    sprites: Res<GangerSprites>,
    facts: GangerVisibilityFacts,
    gangers: Query<(Entity, &Position, &Faction, &LifeState, Option<&DrawnLife>)>,
    mut actors: Query<&mut Visibility, With<GangerSprite>>,
) {
    for (entity, pos, faction, life, drawn) in &gangers {
        let Some(sprite) = sprites.sprite_for(entity) else {
            continue;
        };
        let Ok(mut visibility) = actors.get_mut(sprite) else {
            continue;
        };
        let shown_life = drawn.map_or(*life, |drawn| **drawn);
        set_visibility_quiet(&mut visibility, facts.classify(pos, *faction, shown_life));
    }
}
