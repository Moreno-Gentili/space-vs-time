use crate::ecs::{entities::Position, RouteAction};
use spacetimedb::table;

#[table(name = routables)]
pub struct Routable {
    #[primary_key]
    pub name: String,
    pub counter: u8,
    pub destination: Position,
    pub action: RouteAction
}