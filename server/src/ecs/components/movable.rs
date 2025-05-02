use crate::ecs::entities::{EntityKind,EntityState,Position,Velocity};
use spacetimedb::table;

#[table(name = movables, public)]
pub struct Movable {
    #[primary_key]
    pub name: String,
    pub kind: EntityKind,
    pub state: EntityState,
    pub position: Position,
    pub velocity: Velocity
}

impl Movable {
    pub fn is_player(&self, entity_state: EntityState) -> bool {
        (self.kind == EntityKind::SpacePlayer || self.kind == EntityKind::TimePlayer) && self.state == entity_state
    }

    pub fn can_be_hit(&self) -> bool {
        self.state == EntityState::Moving ||
        self.state == EntityState::ReadyToMove ||
        self.state == EntityState::ReadyToThrow ||
        self.state == EntityState::Throwing
    }
}