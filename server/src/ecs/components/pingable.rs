use spacetimedb::table;

#[table(name = pingables, public)]
pub struct Pingable {
    #[primary_key]
    pub name: String,
    pub timestamp: i64
}