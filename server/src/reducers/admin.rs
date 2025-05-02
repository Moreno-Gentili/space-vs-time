use spacetimedb::{rand::Rng, reducer, Identity, ReducerContext, Table};
use crate::{ecs::{admin::admins, clock::clocks, game::games, identifiable::identifiables, movable::movables, EntityKind, EntityState, GameState, Identifiable, Side, Velocity}, Helpers, GAME_RESET_AT, INFINITE_DURATION };

#[reducer]
pub fn start_game(ctx: &ReducerContext, duration: u8) -> Result<(), String> {
    match its_called_by_an_admin(&ctx) {
        Ok(()) => {
            ensure_has_acceptable_duration(duration)?;
            ctx.set_game_duration(duration);
            reset_game(ctx, GAME_RESET_AT - 2)?;
            Ok(())
        }
        Err(message) => Err(format!("Can not start game: {}", message)),
    }
}

#[reducer]
pub fn start_infinite_game(ctx: &ReducerContext) -> Result<(), String> {
    match its_called_by_an_admin(&ctx) {
        Ok(()) => {
            ctx.set_game_duration(INFINITE_DURATION as u8);
            reset_game(ctx, GAME_RESET_AT - 2)?;
            Ok(())
        }
        Err(message) => Err(format!("Can not start game: {}", message)),
    }
}

#[reducer]
pub fn set_game_auto_start(ctx: &ReducerContext, enable: bool) -> Result<(), String> {
    match its_called_by_an_admin(&ctx) {
        Ok(()) => {
            ctx.set_game_auto_start(enable);
            Ok(())
        }
        Err(message) => Err(format!("Can not set auto start: {}", message)),
    }
}

#[reducer]
pub fn spawn_space_balls(ctx: &ReducerContext, count: u8) -> Result<(), String> {
    match its_called_by_an_admin(&ctx) {
        Ok(()) => {
            ctx.add_random_balls(count.min(30), Side::Space);
            Ok(())
        }
        Err(message) => Err(format!("Can not spawn space balls: {}", message)),
    }
}

#[reducer]
pub fn spawn_time_balls(ctx: &ReducerContext, count: u8) -> Result<(), String> {
    match its_called_by_an_admin(&ctx) {
        Ok(()) => {
            ctx.add_random_balls(count.min(30), Side::Time);
            Ok(())
        }
        Err(message) => Err(format!("Can not spawn time balls: {}", message)),
    }
}

#[reducer]
pub fn spawn_all_balls(ctx: &ReducerContext, count: u8) -> Result<(), String> {
    let n = count.max(1).min(30) as u8;
    match its_called_by_an_admin(&ctx) {
        Ok(()) => {
            ctx.add_random_balls(n, Side::Space);
            ctx.add_random_balls(n, Side::Time);

            Ok(())
        }
        Err(message) => Err(format!("Could not spawn balls randomly: {}", message)),
    }
}

#[reducer]
pub fn clear_identity(ctx: &ReducerContext, name: String) -> Result<(), String> {
    match its_called_by_an_admin(&ctx) {
        Ok(()) => {
            if let Some(mut identifiable) = ctx.find_identifiable(&name) {
                identifiable.identity = Identity::ZERO;
                ctx.db.identifiables().name().update(identifiable);
                Ok(())
            } else {
                Err(format!("Name {} is not identifiable", name))
            }
        }
        Err(message) => Err(format!("Could not spawn balls randomly: {}", message)),
    }
}

#[reducer]
pub fn kick(ctx: &ReducerContext, name: String) -> Result<(), String> {
    match its_called_by_an_admin(&ctx) {
        Ok(()) => {
            if let Some(Identifiable { name, .. }) = ctx.find_identifiable(&name) {
                if let Some(mut movable) = ctx.find_movable(&name) {
                    movable.state = EntityState::Despawning;
                    ctx.db.movables().name().update(movable);
                }
            }

            Ok(())
        }
        Err(message) => Err(format!("Could not spawn balls randomly: {}", message)),
    }
}

#[reducer]
pub fn impulse(ctx: &ReducerContext, name: String, x: f32, y: f32) -> Result<(), String> {
    match its_called_by_an_admin(&ctx) {
        Ok(()) => {
            if let Some(Identifiable { name, .. }) = ctx.find_identifiable(&name) {
                if let Some(_) = ctx.find_movable(&name) {
                    ctx.get_physics_engine().apply_impulse(&name, Velocity { x: x, y: y, z: 0.0 })?;
                }
            }

            Ok(())
        }
        Err(message) => Err(format!("Could not spawn balls randomly: {}", message)),
    }
}

#[reducer]
pub fn impulse_all_balls(ctx: &ReducerContext) -> Result<(), String> {
    match its_called_by_an_admin(&ctx) {
        Ok(()) => {
            for movable in ctx.db.movables().iter() {
                if movable.kind == EntityKind::Ball {
                    ctx.get_physics_engine().apply_impulse(&movable.name, Velocity { x: ctx.rng().gen_range(-0.3..0.3), y: ctx.rng().gen_range(-0.3..0.3), z: 2.0 })?;
                }
            }

            Ok(())
        }
        Err(message) => Err(format!("Could not spawn balls randomly: {}", message)),
    }
}

#[reducer]
pub fn clear_all_identities(ctx: &ReducerContext) -> Result<(), String> {
    match its_called_by_an_admin(&ctx) {
        Ok(()) => {
            for mut identifiable in ctx.db.identifiables().iter() {
                identifiable.identity = Identity::ZERO;
                ctx.db.identifiables().name().update(identifiable);
            }

            Ok(())
        }
        Err(message) => Err(format!("Could not clear all identities: {}", message)),
    }
}

#[reducer]
pub fn remove_all_balls(ctx: &ReducerContext) -> Result<(), String> {
    match its_called_by_an_admin(&ctx) {
        Ok(()) => {
            for mut movable in ctx
                .db
                .movables()
                .iter()
                .filter(|m| m.kind == EntityKind::Ball)
            {
                movable.state = EntityState::Despawning;
                ctx.db.movables().name().update(movable);
            }

            Ok(())
        }
        Err(message) => Err(format!("Could not remove all balls: {}", message)),
    }
}

pub(super) fn its_called_by_an_admin(ctx: &ReducerContext) -> Result<(), String> {
    if ctx.db.admins().identity().find(ctx.sender).is_some() {
        Ok(())
    } else {
        Err(format!("Sender {} is not authorized", ctx.identity()))
    }
}

pub(super) fn reset_game(ctx: &ReducerContext, time: i8) -> Result<(), String> {
    if let Some(mut game) = ctx.get_game() {
        if let Some(mut clock) = ctx.get_clock() {
            game.state = if time > 0 { GameState::GameStarted } else { GameState::Waiting };
            game.remaining = time;
            game.space_score = 0;
            game.time_score = 0;
            game.mvp = String::from("");
            clock.remaning = time as f32;
            ctx.db.games().id().update(game);
            ctx.db.clocks().id().update(clock);
            remove_all_balls(ctx)?;
            Ok(())
        } else {
            Err(format!("Could not reset due to missing clock"))
        }
    } else {
        Err(format!("Could not reset due to missing game"))
    }
}

fn ensure_has_acceptable_duration(duration: u8) -> Result<(), String> {
    if duration == 0 || duration > 99 {
        Err(String::from("Duration must be between 1 and 99"))
    } else {
        Ok(())
    }
}