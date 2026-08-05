use bevy::prelude::*;

crate::support_item! {
    /// Inserted once the battle-running phase has finished.
    #[derive(Resource)]
    struct BattleRunningComplete;
}

pub(crate) fn insert_battle_running_complete(commands: &mut Commands) {
    commands.insert_resource(BattleRunningComplete);
}
