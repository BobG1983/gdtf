mod app_state;
crate::support_use!(app_state::AppState;);

mod running_state;
crate::support_use!(running_state::RunningState;);

mod game_state;
crate::support_use!(game_state::GameState;);

mod battlescape_state;
crate::support_use!(battlescape_state::BattleScapeState;);

mod aftermath_state;
crate::support_use!(aftermath_state::AfterMathState;);
