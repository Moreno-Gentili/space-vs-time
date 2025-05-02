use spacetimedb::{rand::seq::SliceRandom, reducer, table, Identity, ReducerContext, ScheduleAt, Table, TimeDuration, Timestamp};
use crate::{ecs::{clock::clocks, game::{games, Game}, identifiable::identifiables, message::messages, movable::movables, pingable::{pingables, Pingable}, stat::stats, Clock, EntityKind, EntityState, GameState, GameSystem, Identifiable, Message, MovementSystem, RoutingSystem, ScoringSystem, SpawnSystem, Stat, System, UpdateContext}, Helpers, BALL_QUADRANTS, GAME_ID, GAME_RESET_AT, SPACE_USER_NAMES, TIME_USER_NAMES};

#[table(name = schedules, scheduled(update))]
pub struct Schedule {
    #[primary_key]
    #[auto_inc]
    pub scheduled_id: u64,
    pub scheduled_at: ScheduleAt,
}

#[reducer(init)]
pub fn init(ctx: &ReducerContext) {
    schedule_update_reducer(&ctx);
    add_users(&ctx, SPACE_USER_NAMES, EntityKind::SpacePlayer);
    add_users(&ctx, TIME_USER_NAMES, EntityKind::TimePlayer);
    add_game(&ctx, GAME_ID, "");
    add_clock(&ctx, GAME_ID);
    shuffle_ball_quadrants(&ctx);
}

#[reducer(client_connected)]
pub fn identity_connected(_ctx: &ReducerContext) {
    // Do nothing. Players will need to call the join() reducer.
}

#[reducer(client_disconnected)]
pub fn identity_disconnected(ctx: &ReducerContext) {
    if let Some(Identifiable { name, .. }) = ctx.identify() {
        if let Some(mut movable) = ctx.find_movable(&name) {
            movable.state = EntityState::Despawning;
            ctx.db.movables().name().update(movable);
        }
    }
}

#[reducer]
fn update(ctx: &ReducerContext, _arg: Schedule) -> Result<(), String> {
    match its_called_by_server_module(&ctx) {
        Ok(()) => {
            let pipeline = 
            chain::<SpawnSystem>(
            chain::<GameSystem>(
                chain::<RoutingSystem>(
                    chain::<ScoringSystem>(
                        chain::<MovementSystem>(final_handler)))));

            let mut physics_engine = ctx.get_physics_engine();
            let step: f32 = 0.053; // TODO: Get this from a monotonic clock, when it will be available
            let mut update_context = UpdateContext {
                ctx,
                engine: &mut physics_engine,
                step
            };

            pipeline(&mut update_context);

            Ok(())
        }
        Err(message) => Err(format!("Could not call scheduled reducer: {}", message)),
    }
}

fn its_called_by_server_module(ctx: &ReducerContext) -> Result<(), String> {
    if ctx.sender == ctx.identity() {
        Ok(())
    } else {
        Err(format!("Sender {} is not authorized", ctx.identity()))
    }
}

fn schedule_update_reducer(ctx: &ReducerContext) {
    let recurring_schedule = TimeDuration::from_micros(50_000);
    ctx.db.schedules().insert(Schedule {
        scheduled_id: 1,
        scheduled_at: recurring_schedule.into(),
    });
}

fn add_game(ctx: &ReducerContext, id: u8, name: &str) {
    let game = Game {
        id,
        name: String::from(name),
        space_score: 0,
        time_score: 0,
        state: GameState::Waiting,
        remaining: 0,
        mvp: String::from("")
    };

    ctx.db.games().insert(game);
}

fn add_clock(ctx: &ReducerContext, id: u8) {
    let clock = Clock {
        id,
        remaning: if ctx.is_game_auto_starting() { GAME_RESET_AT as f32 } else { 0.0 }
    };
    ctx.db.clocks().insert(clock);
}

fn add_users(ctx: &ReducerContext, names: [&'static str; 8], kind: EntityKind) {
    for name in names {
        add_identifiable(ctx, name, &kind);
        add_pingable(ctx, name);
        add_message(ctx, name);
        add_stat(ctx, name);
    }
}

fn add_identifiable(ctx: &ReducerContext, name: &str, kind: &EntityKind) {
    let identifiable = Identifiable {
        name: String::from(name),
        kind: EntityKind::from(kind.clone()),
        identity: Identity::ZERO,
    };

    ctx.db.identifiables().insert(identifiable);
}

fn add_pingable(ctx: &ReducerContext, name: &str) {
    let pingable = Pingable {
        name: String::from(name),
        timestamp: 0,
    };

    ctx.db.pingables().insert(pingable);
}


fn add_message(ctx: &ReducerContext, name: &str) {
    let message = Message {
        name: String::from(name),
        text: String::from(""),
        timestamp: Timestamp::UNIX_EPOCH
    };

    ctx.db.messages().insert(message);
}

fn add_stat(ctx: &ReducerContext, name: &str) {
    let stat = Stat {
        name: String::from(name),
        points: 0.0
    };

    ctx.db.stats().insert(stat);
}

fn chain<M>(next: impl Fn(&mut UpdateContext)) -> impl Fn(&mut UpdateContext)
where
    M: System
{
    move |mut update_context| {
            M::handle(&mut update_context, &next)
    }
}

fn final_handler(_: &mut UpdateContext) {
}


fn shuffle_ball_quadrants(ctx: &ReducerContext) {
    let mut ball_quadrants = BALL_QUADRANTS.lock().unwrap();
    ball_quadrants.shuffle(&mut ctx.rng());
}