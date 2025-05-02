use spacetimedb::table;

use crate::ecs::GameState;

#[table(name = games, public)]
pub struct Game {
    #[primary_key]
    pub id: u8,
    pub name: String,
    pub space_score: i32,
    pub time_score: i32,
    pub remaining: i8,
    pub state: GameState,
    pub mvp: String
}

impl Game {
    pub fn has_started(&self) -> bool {
        self.state == GameState::GameStarted ||
        self.state == GameState::SpaceScored ||
        self.state == GameState::TimeScored
    }
}