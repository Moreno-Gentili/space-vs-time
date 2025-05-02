mod ecs;
mod services;
mod reducers;
use lazy_static::lazy_static;
use std::sync::{Mutex, MutexGuard};

use ecs::{clock::clocks, components::{
    identifiable::identifiables,
    movable::movables,
    pingable::{pingables, Pingable},
    routable::routables,
    Identifiable, Movable, Routable,
}, game::{games, Game}, message::messages, stat::stats, Clock, EntityKind, EntityState, Message, Position, Side, Stat, Velocity};
use ecs::entities::Dimension;
use spacetimedb::{rand::Rng, ReducerContext, Table};
use services::PhysicsEngine;

const GAME_ID: u8 = 1;
const GAME_AUTO_START_PAUSE_BETWEEN_GAMES: i8 = -9;
const GAME_RESET_AT: i8 = -3;
const GAME_WONT_RESTART: i8 = i8::MIN;

const FIELD_DIMENSION: Dimension = Dimension { x: 24.0, y: 12.0, z: 20.0 };
const PLAYER_DIMENSION: Dimension = Dimension { x: 1.0, y: 1.0, z: 1.6 };

const PLAYER_SPEED: f32 = 2.0;
const PLAYER_BOUNCINESS: f32 = 0.1;
const PLAYER_STALL_DISTANCE: f32 = 0.05;
const PLAYER_STALL_COUNTER: u8 = 20;
const PLAYER_THROW_COUNTER: u8 = 8;
const PLAYER_HIT_COUNTER: u8 = 20;
const PLAYER_SAY_WAIT: f32 = 10.0;
const DEAD_ZONE: f32 = 0.01;

const BALL_RADIUS: f32 = 0.4;
const BALL_BOUNCINESS: f32 = 0.9;
const BALL_SPAWN_MAX_INITIAL_VELOCITY: f32 = 0.5;
const BALL_SPAWN_MIN_HEIGHT: f32 = 8.0;
const BALL_SPAWN_MAX_HEIGHT: f32 = 9.5;

const SPACE_USER_NAMES: [&str; 8] = ["WAN", "FOX", "YUK", "ROG", "KES", "REI", "ASH", "ONO"];
const TIME_USER_NAMES: [&str; 8] = ["LIZ", "ORI", "PRI", "LEO", "DOC", "RAV", "UMA", "TER"];
const FIRST_ROW_INITIAL_POSITION: f32 = 0.65;
const OTHER_ROW_INCREMENT: f32 = 0.25;
const DESTINATION_TOLERANCE: f32 = 0.2;
const INFINITE_DURATION: i8 = i8::MAX;

const SURE_CATCH_RELATIVE_SPEED: f32 = 10.0;
const SURE_HIT_RELATIVE_SPEED: f32 = 15.0;

const HIT_IMPULSE_MULTIPLIER: f32 = 50.0;

lazy_static! {
    static ref PHYSICS_ENGINE: Mutex<PhysicsEngine> = Mutex::new(PhysicsEngine::new(FIELD_DIMENSION));
    static ref BALL_QUADRANTS: Mutex<Vec<u8>> = Mutex::new((0..30).collect());
    static ref BALL_QUADRANT_INDEX: Mutex<usize> = Mutex::new(0);
    static ref BALL_ID: Mutex<u32> = Mutex::new(0);
    static ref SPAWN_PLAYERS_WITH_BALL: Mutex<bool> = Mutex::new(false);
    static ref GAME_AUTO_START: Mutex<bool> = Mutex::new(true);
    static ref GAME_DURATION: Mutex<u8> = Mutex::new(20);
}

trait Helpers {
    fn identify(&self) -> Option<Identifiable>;
    fn find_identifiable(&self, name: &str) -> Option<Identifiable>;
    fn find_movable(&self, name: &str) -> Option<Movable>;
    fn find_routable(&self, name: &str) -> Option<Routable>;
    fn find_pingable(&self, name: &str) -> Option<Pingable>;
    fn find_message(&self, name: &str) -> Option<Message>;
    fn find_stat(&self, name: &str) -> Option<Stat>;
    fn get_game(&self) -> Option<Game>;
    fn get_clock(&self) -> Option<Clock>;
    fn get_physics_engine(&self) -> MutexGuard<'static, PhysicsEngine>;
    fn get_player_position(&self, slot: usize, side: Side) -> Position;
    fn add_random_balls_for_each_player(&self);
    fn add_random_balls(&self, count: u8, side: Side);
    fn add_player_at_next_position(&self, name: &str, side: Side);
    fn add_ball(&self, position: Position, velocity: Velocity) -> String;
    fn get_entity_kind_from_player_name(&self, name: &str) -> Result<EntityKind, String>;
    fn are_players_spawning_with_ball(&self) -> bool;
    fn set_spawn_players_with_ball(&self, enable: bool);
    fn is_game_auto_starting(&self) -> bool;
    fn set_game_auto_start(&self, enable: bool);
    fn get_game_duration(&self) -> u8;
    fn set_game_duration(&self, duration: u8);
    fn with_user_component<T, F, R>(
        &self,
        lookup: fn(&ReducerContext, &str) -> Option<T>,
        op: F,
    ) -> Result<R, String>
    where
        F: FnOnce(T) -> Result<R, String>;
}

impl Helpers for ReducerContext {
    fn identify(&self) -> Option<Identifiable> {
        self.db.identifiables().identity().filter(self.sender).next()
    }

    fn find_identifiable(&self, name: &str) -> Option<Identifiable> {
        let normalized_name = name.to_uppercase();
        self.db.identifiables().name().find(String::from(normalized_name))
    }

    fn find_movable(&self, name: &str) -> Option<Movable> {
        self.db.movables().name().find(String::from(name))
    }

    fn find_routable(&self, name: &str) -> Option<Routable> {
        self.db.routables().name().find(String::from(name))
    }

    fn find_message(&self, name: &str) -> Option<Message> {
        self.db.messages().name().find(String::from(name))
    }

    fn find_pingable(&self, name: &str) -> Option<Pingable> {
        self.db.pingables().name().find(String::from(name))
    }

    fn find_stat(&self, name: &str) -> Option<Stat> {
        self.db.stats().name().find(String::from(name))
    }

    fn get_game(&self) -> Option<Game> {
        match self.db.games().id().find(GAME_ID) {
            Some(game) => Some(game),
            None => {
                log::error!("Could not find game");
                None
            }
        }
    }

    fn get_entity_kind_from_player_name(&self, name: &str) -> Result<EntityKind, String> {
        if SPACE_USER_NAMES.contains(&name) {
            Ok(EntityKind::SpacePlayer)
        } else if TIME_USER_NAMES.contains(&name) {
            Ok(EntityKind::TimePlayer)
        } else {
            Err(format!("Could not find user {}", name))
        }
    }

    fn get_clock(&self) -> Option<Clock> {
        match self.db.clocks().id().find(GAME_ID) {
            Some(clock) => Some(clock),
            None => {
                log::error!("Could not find clock");
                None
            }
        }
    }

    fn get_player_position(&self, slot: usize, side: Side) -> Position {
        let sign_x = if side == Side::Space { -1.0 } else { 1.0 };
        let x = FIELD_DIMENSION.x / 2.0
            * sign_x
            * (FIRST_ROW_INITIAL_POSITION + (OTHER_ROW_INCREMENT * (slot / 4) as f32));
        const Y_STEP: f32 = FIELD_DIMENSION.y * 0.95 / 8.0;
        let y_positions: Vec<f32> = vec![Y_STEP * 0.5, -Y_STEP * 1.5, Y_STEP * 2.5, -Y_STEP * 3.5];
        let y = y_positions[(slot % 4) as usize] as f32
            + (Y_STEP * (slot / 4) as f32)
            - Y_STEP / 6.0;
        Position { x, y, z: 0.0 }
    }

    fn add_player_at_next_position(&self, name: &str, side: Side) {
        let kind = if side == Side::Space { EntityKind::SpacePlayer } else { EntityKind::TimePlayer };
        let position = get_next_player_init_position(self, side);
        self.db.movables().insert(Movable {
            name: String::from(name),
            kind,
            state: EntityState::Spawning,
            position,
            velocity: Velocity::ZERO
        });
    }

    fn add_random_balls_for_each_player(&self) {
        for player in self.db.movables().iter() {
            match player.kind {
                EntityKind::SpacePlayer => self.add_random_balls(1, Side::Space),
                EntityKind::TimePlayer => self.add_random_balls(1, Side::Time),
                _ => { }
            }
        }
    }

    fn add_random_balls(&self, count: u8, side: Side) {
        for _ in 0..count {
            let position = get_ball_init_position(self, side.clone());
            let velocity = get_random_ball_velocity(self);
            self.add_ball(position, velocity);
        }
    }

    fn add_ball(&self, position: Position, velocity: Velocity) -> String {
        let ball_name = format!("Ball{}", get_next_ball_id());
        self.db.movables().insert(
            Movable {
                name: ball_name.clone(),
                state: EntityState::Spawning,
                kind: EntityKind::Ball,
                position,
                velocity
            });

        ball_name
    }

    fn get_physics_engine(&self) -> MutexGuard<'static, PhysicsEngine> {
        PHYSICS_ENGINE.lock().unwrap()
    }

    fn are_players_spawning_with_ball(&self) -> bool {
        let value = SPAWN_PLAYERS_WITH_BALL.lock().unwrap();
        *value
    }

    fn set_spawn_players_with_ball(&self, enable: bool) {
        let mut value = SPAWN_PLAYERS_WITH_BALL.lock().unwrap();
        *value = enable;
    }

    fn is_game_auto_starting(&self) -> bool {
        let value = GAME_AUTO_START.lock().unwrap();
        *value
    }

    fn set_game_auto_start(&self, enable: bool) {
        let mut value = GAME_AUTO_START.lock().unwrap();
        *value = enable;
    }

    fn get_game_duration(&self) -> u8 {
        let value = GAME_DURATION.lock().unwrap();
        *value
    }

    fn set_game_duration(&self, duration: u8) {
        let mut value = GAME_DURATION.lock().unwrap();
        *value = duration;
    }

    fn with_user_component<T, F, R>(
        &self,
        lookup: fn(&ReducerContext, &str) -> Option<T>,
        op: F,
    ) -> Result<R, String>
    where
        F: FnOnce(T) -> Result<R, String>,
    {
        if let Some(Identifiable { name, .. }) = self.identify() {
            if let Some(entity) = lookup(self, &name) {
                op(entity)
            } else {
                Err(format!("Could not find component {}", name))
            }
        } else {
            Err(format!("Could not identify user {}", self.sender))
        }
    }
}

fn get_ball_init_position(ctx: &ReducerContext, side: Side) -> Position {
    let x_quadrants: u8 = 5;
    let y_quadrants: u8 = 6;
    let quadrant = get_next_ball_quadrant();
    let col = quadrant % x_quadrants;
    let row = quadrant / x_quadrants;
    let x_offset = if side == Side::Space { -FIELD_DIMENSION.x * 0.5 } else { 0.0 };
    let x = (FIELD_DIMENSION.x * 0.5 / (x_quadrants as f32)) * (col as f32) + x_offset
        + (FIELD_DIMENSION.x * 0.5 / ((x_quadrants as f32) * 2.0));
    let y = (FIELD_DIMENSION.y / (y_quadrants as f32)) * (row as f32) - (FIELD_DIMENSION.y * 0.5)
        + (FIELD_DIMENSION.y * 0.5 / ((y_quadrants as f32) * 2.0));
    Position { x, y, z: ctx.rng().gen_range(BALL_SPAWN_MIN_HEIGHT..BALL_SPAWN_MAX_HEIGHT) }
}

fn get_next_player_init_position(ctx: &ReducerContext, side: Side) -> Position {
    let player_kind = if side == Side::Space { EntityKind::SpacePlayer } else { EntityKind::TimePlayer };
    let slot = ctx
            .db
            .movables()
            .iter()
            .filter(|m| m.kind == player_kind)
            .count();

    ctx.get_player_position(slot, side)
}

fn get_random_ball_velocity(ctx: &ReducerContext) -> Velocity {
    Velocity {
        x: ctx.rng().gen_range(-BALL_SPAWN_MAX_INITIAL_VELOCITY..BALL_SPAWN_MAX_INITIAL_VELOCITY),
        y: ctx.rng().gen_range(-BALL_SPAWN_MAX_INITIAL_VELOCITY..BALL_SPAWN_MAX_INITIAL_VELOCITY),
        z: 0.0 }
}

fn get_next_ball_id() -> u32 {
    let mut ball_id = BALL_ID.lock().unwrap();
    *ball_id += 1;
    return *ball_id;
}

fn get_next_ball_quadrant() -> u8 {
    let ball_quadrants = BALL_QUADRANTS.lock().unwrap();
    let mut index = BALL_QUADRANT_INDEX.lock().unwrap();
    *index = index.checked_add(1).unwrap() % ball_quadrants.len();
    return ball_quadrants[*index];
}