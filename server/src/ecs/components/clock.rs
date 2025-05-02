use spacetimedb::table;

#[table(name = clocks)]
pub struct Clock {
    #[primary_key]
    pub id: u8,
    pub remaning: f32
}