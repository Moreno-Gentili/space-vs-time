use spacetimedb::{table, Identity};

#[table(name = admins)]
pub struct Admin {
    #[primary_key]
    pub identity: Identity
}