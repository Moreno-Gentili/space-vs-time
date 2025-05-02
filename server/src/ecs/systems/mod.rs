mod movement_system;
mod routing_system;
mod game_system;
mod spawn_system;
mod scoring_system;

use spacetimedb::ReducerContext;
pub use movement_system::MovementSystem;
pub use routing_system::RoutingSystem;
pub use game_system::GameSystem;
pub use spawn_system::SpawnSystem;
pub use scoring_system::ScoringSystem;

use crate::services::PhysicsEngine;

pub trait System {
    fn handle(req: &mut UpdateContext, next: impl Fn(&mut UpdateContext));
}

pub struct UpdateContext<'a> {
    pub ctx: &'a ReducerContext,
    pub engine: &'a mut PhysicsEngine,
    pub step: f32
}