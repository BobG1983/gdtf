use bevy::prelude::*;

crate::support_item! {
                                                                            #[derive(Resource)]
    struct BattleRunningComplete;
}

pub(in crate::states::running::game::battlescape) fn insert_battle_running_complete(
    commands: &mut Commands,
) {
    commands.insert_resource(BattleRunningComplete);
}
