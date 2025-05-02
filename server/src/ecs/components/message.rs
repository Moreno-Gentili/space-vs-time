use spacetimedb::{table, Timestamp};

#[table(name = messages, public)]
pub struct Message {
    #[primary_key]
    pub name: String,
    pub text: String,
    pub timestamp: Timestamp
}