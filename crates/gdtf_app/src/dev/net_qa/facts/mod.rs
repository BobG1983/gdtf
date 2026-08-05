pub(crate) mod battle_activity;
pub(crate) mod game_facts;
pub(crate) mod read;

crate::support_use!(battle_activity::BattleActivity;);
crate::support_use!(game_facts::GameFacts;);
crate::support_use!(read::GameFactsParam;);

#[cfg(test)]
mod test;
