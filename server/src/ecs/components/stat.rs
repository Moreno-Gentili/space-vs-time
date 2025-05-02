use spacetimedb::table;

#[table(name = stats)]
pub struct Stat {
    #[primary_key]
    pub name: String,
    pub points: f32
}