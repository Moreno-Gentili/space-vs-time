use crate::ecs::entities::*;
use spacetimedb::{table, Identity};

#[table(name = identifiables)]
pub struct Identifiable {
    #[primary_key]
    pub name: String,
    pub kind: EntityKind,
    #[index(btree)]
    pub identity: Identity
}