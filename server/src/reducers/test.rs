// This file is not needed by the game and can be safely removed
use spacetimedb::{reducer, ReducerContext, Table};

use crate::{ecs::{movable::movables, routable::routables, EntityKind, EntityState, Movable, Position, Routable, RouteAction, Side, Velocity}, get_next_ball_id, Helpers, BALL_RADIUS, FIELD_DIMENSION, INFINITE_DURATION, PLAYER_BOUNCINESS, PLAYER_DIMENSION, SPACE_USER_NAMES, TIME_USER_NAMES};
use super::admin::{clear_all_identities, its_called_by_an_admin, reset_game};

#[reducer]
pub fn test(ctx: &ReducerContext, name: String) -> Result<(), String> {
    clear_everything(ctx)?;
    match its_called_by_an_admin(&ctx) {
        Ok(()) => {
            match name.as_str() {
                "0" => stage_0(ctx),
                "1" => stage_1(ctx),
                "2" => stage_2(ctx),
                "ball_hit_1" => test_ball_hit_1(ctx),
                "ball_hit_2" => test_ball_hit_2(ctx),
                "ball_hit_3" => test_ball_hit_3(ctx),
                "ball_hit_4" => test_ball_hit_4(ctx),
                "ball_hit_5" => test_ball_hit_5(ctx),
                "ball_hit_6" => test_ball_hit_6(ctx),
                "ball_hit_7" => test_ball_hit_7(ctx),
                "ball_rebound_1" => test_ball_rebound_1(ctx),
                "ball_rebound_2" => test_ball_rebound_2(ctx),
                _ => {
                    Err(format!("Test {} does not exist", name))
                }
            }
        }
        Err(message) => Err(format!("Can not test {}: {}", name, message)),
    }
}

fn stage_0(ctx: &ReducerContext) -> Result<(), String> {
    ctx.set_spawn_players_with_ball(false);
    reset_game(ctx, INFINITE_DURATION)
}

fn stage_1(ctx: &ReducerContext) -> Result<(), String> {
    ctx.set_spawn_players_with_ball(true);
    reset_game(ctx, INFINITE_DURATION)
}

fn stage_2(ctx: &ReducerContext) -> Result<(), String> {
    ctx.set_spawn_players_with_ball(false);
    reset_game(ctx, INFINITE_DURATION)?;
    for _ in 0..8 {
        ctx.add_random_balls(1, Side::Space);
        ctx.add_random_balls(1, Side::Time);
    }
    Ok(())
}

fn test_ball_hit_1(ctx: &ReducerContext) -> Result<(), String> {
    let x = FIELD_DIMENSION.x / 10.0;
    let space_player_name = SPACE_USER_NAMES[0];
    let time_player_name = TIME_USER_NAMES[0];
    add_test_player(ctx, space_player_name, Position { x: -x, y: 0.0, z: 0.0 }, EntityState::ReadyToThrow)?;
    add_test_player(ctx, time_player_name, Position { x: x, y: 0.0, z: 0.0 }, EntityState::ReadyToMove)?;
    let ball_name = add_test_ball(ctx, Position { x: -x, y: 0.0, z: PLAYER_DIMENSION.z }, EntityState::ReadyToThrow);

    let mut engine = ctx.get_physics_engine();
    engine.catch_ball(space_player_name, &ball_name)?;

    throw_test_ball(ctx, space_player_name, Position { x: x, y: 0.0, z: 0.0 });

    Ok(())
}

fn test_ball_hit_2(ctx: &ReducerContext) -> Result<(), String> {
    let x = FIELD_DIMENSION.x / 5.0;
    let space_player_name = SPACE_USER_NAMES[0];
    let time_player_name = TIME_USER_NAMES[0];
    add_test_player(ctx, space_player_name, Position { x: -x, y: 0.0, z: 0.0 }, EntityState::ReadyToThrow)?;
    add_test_player(ctx, time_player_name, Position { x: x, y: 0.0, z: 0.0 }, EntityState::ReadyToMove)?;
    let ball_name = add_test_ball(ctx, Position { x: -x, y: 0.0, z: PLAYER_DIMENSION.z }, EntityState::ReadyToThrow);

    let mut engine = ctx.get_physics_engine();
    engine.catch_ball(space_player_name, &ball_name)?;

    // throw_test_ball(ctx, space_player_name, Position::ZERO);
    throw_test_ball(ctx, space_player_name, Position { x: x, y: 0.0, z: 0.0 });

    Ok(())
}

fn test_ball_hit_3(ctx: &ReducerContext) -> Result<(), String> {
    let x = FIELD_DIMENSION.x / 2.3;
    let space_player_name = SPACE_USER_NAMES[0];
    let time_player_name = TIME_USER_NAMES[0];
    add_test_player(ctx, space_player_name, Position { x: -x, y: 0.0, z: 0.0 }, EntityState::ReadyToThrow)?;
    add_test_player(ctx, time_player_name, Position { x: x, y: 0.0, z: 0.0 }, EntityState::ReadyToMove)?;
    let ball_name = add_test_ball(ctx, Position { x: -x, y: 0.0, z: PLAYER_DIMENSION.z }, EntityState::ReadyToThrow);

    let mut engine = ctx.get_physics_engine();
    engine.catch_ball(space_player_name, &ball_name)?;

    // throw_test_ball(ctx, space_player_name, Position::ZERO);
    throw_test_ball(ctx, space_player_name, Position { x: x, y: 0.0, z: 0.0 });

    Ok(())
}

fn test_ball_hit_4(ctx: &ReducerContext) -> Result<(), String> {
    let x = FIELD_DIMENSION.x / 2.3;
    let y = FIELD_DIMENSION.y * 0.45;
    let space_player_name = SPACE_USER_NAMES[0];
    let time_player_name = TIME_USER_NAMES[0];
    add_test_player(ctx, space_player_name, Position { x: -x, y: y, z: 0.0 }, EntityState::ReadyToThrow)?;
    add_test_player(ctx, time_player_name, Position { x: x, y: -y, z: 0.0 }, EntityState::ReadyToMove)?;
    let ball_name = add_test_ball(ctx, Position { x: -x, y: y, z: PLAYER_DIMENSION.z }, EntityState::ReadyToThrow);

    let mut engine = ctx.get_physics_engine();
    engine.catch_ball(space_player_name, &ball_name)?;

    // throw_test_ball(ctx, space_player_name, Position::ZERO);
    throw_test_ball(ctx, space_player_name, Position { x: x, y: -y, z: 0.0 });

    Ok(())
}

fn test_ball_hit_5(ctx: &ReducerContext) -> Result<(), String> {
    let x = FIELD_DIMENSION.x * 0.3;
    let y = 0.0;
    let space_player_name = SPACE_USER_NAMES[0];
    let time_player_name = TIME_USER_NAMES[0];
    add_test_player(ctx, space_player_name, Position { x: -x, y: y, z: 0.0 }, EntityState::ReadyToThrow)?;
    add_test_player(ctx, time_player_name, Position { x: x, y: -y, z: 0.0 }, EntityState::ReadyToThrow)?;
    let ball_name_1 = add_test_ball(ctx, Position { x: -x, y: y, z: PLAYER_DIMENSION.z }, EntityState::ReadyToThrow);
    let ball_name_2 = add_test_ball(ctx, Position { x: x, y: y, z: PLAYER_DIMENSION.z }, EntityState::ReadyToThrow);

    let mut engine = ctx.get_physics_engine();
    engine.catch_ball(space_player_name, &ball_name_1)?;
    engine.catch_ball(time_player_name, &ball_name_2)?;

    // throw_test_ball(ctx, space_player_name, Position::ZERO);
    throw_test_ball(ctx, space_player_name, Position { x: x, y: -y, z: 0.0 });

    Ok(())
}

fn test_ball_hit_6(ctx: &ReducerContext) -> Result<(), String> {
    let x = FIELD_DIMENSION.x * 0.3;
    let y = 0.0;
    let space_player_name = SPACE_USER_NAMES[0];
    let time_player_name_1 = TIME_USER_NAMES[0];
    let time_player_name_2 = TIME_USER_NAMES[1];
    add_test_player(ctx, space_player_name, Position { x: -x, y: y, z: 0.0 }, EntityState::ReadyToThrow)?;
    add_test_player(ctx, time_player_name_1, Position { x: x, y: -y, z: 0.0 }, EntityState::ReadyToMove)?;
    add_test_player(ctx, time_player_name_2, Position { x: x + PLAYER_DIMENSION.x, y: -y, z: 0.0 }, EntityState::ReadyToMove)?;
    let ball_name = add_test_ball(ctx, Position { x: -x, y: y, z: PLAYER_DIMENSION.z }, EntityState::ReadyToThrow);

    let mut engine = ctx.get_physics_engine();
    engine.catch_ball(space_player_name, &ball_name)?;

    // throw_test_ball(ctx, space_player_name, Position::ZERO);
    throw_test_ball(ctx, space_player_name, Position { x: x + 2.0, y: -y, z: 0.0 });

    Ok(())
}

fn test_ball_hit_7(ctx: &ReducerContext) -> Result<(), String> {
    let x = FIELD_DIMENSION.x / 2.0 - PLAYER_DIMENSION.x;
    let space_player_name = SPACE_USER_NAMES[0];
    let time_player_name = TIME_USER_NAMES[0];
    add_test_player(ctx, space_player_name, Position { x: -x, y: 0.0, z: 0.0 }, EntityState::ReadyToThrow)?;
    add_test_player(ctx, time_player_name, Position { x: x, y: 0.0, z: 0.0 }, EntityState::ReadyToMove)?;
    let ball_name = add_test_ball(ctx, Position { x: -x, y: 0.0, z: PLAYER_DIMENSION.z }, EntityState::ReadyToThrow);

    let mut engine = ctx.get_physics_engine();
    engine.catch_ball(space_player_name, &ball_name)?;

    throw_test_ball(ctx, space_player_name, Position { x: x, y: 0.0, z: 0.0 });

    Ok(())
}

fn test_ball_rebound_1(ctx: &ReducerContext) -> Result<(), String> {
    let x = FIELD_DIMENSION.x / 5.0;
    let space_player_name = SPACE_USER_NAMES[0];
    let time_player_name = TIME_USER_NAMES[0];
    add_test_player(ctx, space_player_name, Position { x: -x, y: 0.0, z: 0.0 }, EntityState::ReadyToThrow)?;
    add_test_player(ctx, time_player_name, Position { x: x, y: 0.0, z: 0.0 }, EntityState::ReadyToMove)?;
    let ball_name = add_test_ball(ctx, Position { x: -x, y: 0.0, z: PLAYER_DIMENSION.z }, EntityState::ReadyToThrow);

    let mut engine = ctx.get_physics_engine();
    engine.catch_ball(space_player_name, &ball_name)?;

    // throw_test_ball(ctx, space_player_name, Position::ZERO);
    throw_test_ball(ctx, space_player_name, Position { x: x * 0.5, y: 0.0, z: 0.0 });

    Ok(())
}

fn test_ball_rebound_2(ctx: &ReducerContext) -> Result<(), String> {
    let x = FIELD_DIMENSION.x / 5.0;
    let space_player_name = SPACE_USER_NAMES[0];
    let time_player_name = TIME_USER_NAMES[0];
    add_test_player(ctx, space_player_name, Position { x: -x, y: 0.0, z: 0.0 }, EntityState::ReadyToThrow)?;
    add_test_player(ctx, time_player_name, Position { x: x * 2.0, y: 0.0, z: 0.0 }, EntityState::ReadyToMove)?;
    let ball_name = add_test_ball(ctx, Position { x: -x, y: 0.0, z: PLAYER_DIMENSION.z }, EntityState::ReadyToThrow);

    let mut engine = ctx.get_physics_engine();
    engine.catch_ball(space_player_name, &ball_name)?;

    // throw_test_ball(ctx, space_player_name, Position::ZERO);
    throw_test_ball(ctx, space_player_name, Position { x: FIELD_DIMENSION.x * 0.6, y: 0.0, z: 0.0 });

    Ok(())
}

fn add_test_player(ctx: &ReducerContext, name: &str, position: Position, state: EntityState) -> Result<(), String> {
    let entity_kind = ctx.get_entity_kind_from_player_name(name)?;
    let player = Movable {
        name: String::from(name),
        kind: entity_kind,
        state,
        position: position,
        velocity: Velocity::ZERO };
    
    let mut engine = ctx.get_physics_engine();
    engine.upsert_player(&player, PLAYER_DIMENSION, PLAYER_BOUNCINESS);
    ctx.db.movables().insert(player);
    Ok(())
}

fn add_test_ball(ctx: &ReducerContext, position: Position, state: EntityState) -> String {
    let ball_id = get_next_ball_id();
    let ball_name = format!("Ball{}", ball_id);
    let ball = Movable {
        name: ball_name.clone(),
        kind: EntityKind::Ball,
        state,
        position: position,
        velocity: Velocity::ZERO };
    
    let mut engine = ctx.get_physics_engine();
    engine.upsert_ball(&ball, BALL_RADIUS, BALL_RADIUS);
    ctx.db.movables().insert(ball);
    ball_name
}

fn throw_test_ball(ctx: &ReducerContext, player_name: &str, target: Position) {
    ctx.db.routables().insert(Routable {
        name: String::from(player_name),
        counter: 0,
        destination: target,
        action: RouteAction::Throw,
    });
}

fn clear_everything(ctx: &ReducerContext) -> Result<(), String> {
    clear_entities(ctx)?;
    clear_all_identities(ctx)?;
    reset_game(ctx, INFINITE_DURATION)?;
    Ok(())
}

fn clear_entities(ctx: &ReducerContext) -> Result<(), String> {
    let mut engine = ctx.get_physics_engine();
    for movable in ctx.db.movables().iter() {
        engine.delete(&movable.name)?;
        ctx.db.movables().delete(movable);
    }

    for routable in ctx.db.routables().iter() {
        ctx.db.routables().delete(routable);
    }

    Ok(())
}