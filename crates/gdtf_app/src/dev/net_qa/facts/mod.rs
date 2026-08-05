pub(crate) mod battle_activity;
pub(crate) mod battle_model;
pub(crate) mod battle_screen;
pub(crate) mod game_facts;
pub(crate) mod presenter_readiness;
pub(crate) mod read;

crate::support_use!(battle_activity::BattleActivity;);
crate::support_use!(battle_model::BattleModel;);
crate::support_use!(battle_screen::BattleScreen;);
crate::support_use!(game_facts::GameFacts;);
crate::support_use!(presenter_readiness::PresenterReadiness;);
crate::support_use!(read::GameFactsParam;);

#[cfg(test)]
mod test;
