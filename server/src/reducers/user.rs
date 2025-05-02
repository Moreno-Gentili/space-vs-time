use spacetimedb::{reducer, Identity, ReducerContext, Table, Timestamp};
use crate::{ecs::{identifiable::identifiables, message::messages, pingable::pingables, routable::routables, EntityState, Position, Routable, RouteAction, Side}, Helpers, FIELD_DIMENSION, PLAYER_SAY_WAIT, SPACE_USER_NAMES };

#[reducer]
pub fn join(ctx: &ReducerContext, name: String) -> Result<(), String> {
    match ensure_user_name_belongs_to_me(&ctx, &name) {
        Ok(name) => {
            if ctx.find_movable(&name).is_none() {
                let side = if SPACE_USER_NAMES.contains(&name.as_str()) { Side::Space } else { Side::Time };
                ctx.add_player_at_next_position(&name, side);
            }
            Ok(())
        }
        Err(message) => Err(format!("Could not join: {}", message)),
    }
}

#[reducer]
pub fn say(ctx: &ReducerContext, text: String) -> Result<(), String> {
    ctx.with_user_component(
        ReducerContext::find_message,
        |mut message| {
            ensure_has_joined(ctx)?;
            ensure_not_too_long(&text)?;
            ensure_not_spamming(ctx.timestamp, message.timestamp)?;

            message.text = text;
            message.timestamp = ctx.timestamp;
            ctx.db.messages().name().update(message);
            Ok(())
        },
    )
}

#[reducer]
pub fn ping(ctx: &ReducerContext, timestamp: i64) -> Result<(), String> {
    ctx.with_user_component(
        ReducerContext::find_pingable,
        |mut pingable| {
            pingable.timestamp = timestamp;
            ctx.db.pingables().name().update(pingable);
            Ok(())
        },
    )
}

#[reducer]
pub fn move_to(ctx: &ReducerContext, x: f32, y: f32) -> Result<(), String> {
    ctx.with_user_component(
        ReducerContext::find_movable,
        |movable| {
            ensure_within_bounds(x, y)?;
            ensure_game_has_started(ctx)?;
            ensure_has_joined(ctx)?;

            if movable.is_player(EntityState::ReadyToMove) || (movable.is_player(EntityState::Spawning) && !ctx.are_players_spawning_with_ball()) {
                if ctx.find_routable(&movable.name.as_str()).is_none() {
                    ctx.db.routables().insert(Routable {
                        name: movable.name.clone(),
                        counter: 0,
                        destination: get_ground_position(x, y),
                        action: RouteAction::Move
                    });
                }

                Ok(())
            } else {
                Err(format!("{} cannot move while in the {:?} state. Must be in the {:?} state.",
                movable.name,
                if movable.state == EntityState::Spawning { EntityState::ReadyToThrow } else { movable.state },
                EntityState::ReadyToMove))
            }
        },
    )
}

#[reducer]
pub fn throw_to(ctx: &ReducerContext, x: f32, y: f32) -> Result<(), String> {
    ctx.with_user_component(
        ReducerContext::find_movable,
        |movable| {
            ensure_within_bounds(x, y)?;
            ensure_game_has_started(ctx)?;
            ensure_has_joined(ctx)?;

            if movable.is_player(EntityState::ReadyToThrow) || (movable.is_player(EntityState::Spawning) && ctx.are_players_spawning_with_ball()) {
                if ctx.find_routable(&movable.name.as_str()).is_none() {
                    ctx.db.routables().insert(Routable {
                        name: movable.name.clone(),
                        counter: 0,
                        destination: get_ground_position(x, y),
                        action: RouteAction::Throw
                    });
                }
                Ok(())
            } else {
                Err(format!("{} cannot move while in the {:?} state. Must be in the {:?} state.",
                movable.name,
                if movable.state == EntityState::Spawning { EntityState::ReadyToMove } else { movable.state },
                EntityState::ReadyToThrow))
            }
        },
    )
}

fn ensure_user_name_belongs_to_me(ctx: &ReducerContext, name: &str) -> Result<String, String> {
    if let Some(mut identifiable) = ctx.find_identifiable(&name) {
        let name = String::from(identifiable.name.as_str());
        if identifiable.identity == Identity::ZERO {
            // Claim this user for myself
            identifiable.identity = ctx.sender;
            ctx.db.identifiables().name().update(identifiable);
            Ok(name)
        } else if identifiable.identity == ctx.sender {
            Ok(name)
        } else {
            Err(format!(
                "User {} tried to claim player {}",
                ctx.sender, name
            ))
        }
    } else {
        Err(format!("Sender {} not authorized", ctx.sender))
    }
}

fn ensure_has_joined(ctx: &ReducerContext) -> Result<(), String> {
    if let Some(identity) = ctx.identify() {
        if let Some(_) = ctx.find_movable(&identity.name) {
            Ok(())
        } else {
            Err(String::from("You must join first"))
        }
    } else {
        Err(String::from("Could not identify you"))
    }
}

fn ensure_not_too_long(text: &str) -> Result<(), String> {
    if text.len() <= 12 {
        Ok(())
    } else {
        Err(String::from("Text is too long. Max is 12 characters."))
    }
}

fn ensure_not_spamming(current: Timestamp, previous: Timestamp) -> Result<(), String> {
    if let Some(duration) = current.duration_since(previous) {
        let remaining = PLAYER_SAY_WAIT - duration.as_secs_f32();
        if remaining < 0.0 {
            Ok(())
        } else {
            Err(format!("Retry in {}s", remaining))
        }
    } else {
        Err(String::from("Could not determine the time passed since last execution"))
    }
}

fn get_ground_position(x: f32, y: f32) -> Position {
    Position {
        x: x.max(-FIELD_DIMENSION.x / 2.0).min(FIELD_DIMENSION.x / 2.0),
        y: y.max(-FIELD_DIMENSION.y / 2.0).min(FIELD_DIMENSION.y / 2.0),
        // z: PLAYER_DIMENSION.z / 4.0.
        z: 0.0
    }
}

fn ensure_within_bounds(x: f32, y: f32) -> Result<(), String> {
    let min_x = FIELD_DIMENSION.x / -2.0;
    let max_x = FIELD_DIMENSION.x / 2.0;
    let min_y = FIELD_DIMENSION.y / -2.0;
    let max_y = FIELD_DIMENSION.y / 2.0;

    if x < min_x || x > max_x || y < min_y || y > max_y {
        Err(format!("Coordinates {},{} are out of bounds. Must stay within {},{} and {},{}", x, y, min_x, min_y, max_x, max_y))
    } else {
        Ok(())
    }
}

fn ensure_game_has_started(ctx: &ReducerContext) -> Result<(), String> {
    match ctx.get_game() {
        Some(game) if game.has_started() => Ok(()),
        _ => Err(String::from("Game has not started yet"))
    }
}