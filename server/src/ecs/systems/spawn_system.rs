use spacetimedb::{ReducerContext, Table};
use crate::{ecs::{game::{games, Game}, movable::movables, routable::routables, EntityKind, EntityState, GameState, Movable, Position, Side, Velocity}, services::PhysicsEngine, Helpers, BALL_BOUNCINESS, BALL_RADIUS, PLAYER_BOUNCINESS, PLAYER_DIMENSION };
use super::{System, UpdateContext};

pub struct SpawnSystem;

impl System for SpawnSystem {
    fn handle(context: &mut UpdateContext, next: impl Fn(&mut UpdateContext)) {
        Self::spawn_or_despawn_movables(context.ctx, context.engine);
        next(context);
        Self::reset_movables_for_new_match_if_needed(context.ctx, context.engine);
    }
}

impl SpawnSystem {
    fn spawn_or_despawn_movables(ctx: &ReducerContext, engine: &mut PhysicsEngine) {
        for movable in ctx.db.movables().iter() {
            match movable.state {
                EntityState::Spawning => {
                    match movable.kind {
                        EntityKind::SpacePlayer | EntityKind::TimePlayer => Self::spawn_player(ctx, engine, movable),
                        EntityKind::Ball => Self::spawn_ball(ctx, engine, movable)
                    }
                },
                EntityState::Despawning => {
                    match movable.kind {
                        EntityKind::SpacePlayer | EntityKind::TimePlayer => Self::despawn_player(ctx, engine, &movable.name),
                        EntityKind::Ball => Self::despawn_ball(ctx, engine, &movable.name)
                    }
                },
                _ => {}
            }
        }
    }

    fn spawn_player(ctx: &ReducerContext, engine: &mut PhysicsEngine, player: Movable) {
        if let Some(game) = ctx.get_game() {
            Self::reset_player(ctx, engine, player, &game);
        }
    }

    fn spawn_ball(ctx: &ReducerContext, physics_engine: &mut PhysicsEngine, mut ball: Movable) {
        ball.state = EntityState::Moving;
        physics_engine.upsert_ball(&ball, BALL_RADIUS, BALL_BOUNCINESS);
        ctx.db.movables().name().update(ball);
    }
    
    fn reset_movables_for_new_match_if_needed(ctx: &ReducerContext, engine: &mut PhysicsEngine) {
        let maybe_game = ctx.get_game();
        if maybe_game.is_none() {
            return;
        }

        let mut game = maybe_game.unwrap();
        if game.state != GameState::Reset {
            return;
        }

        let mut space_player_count: usize = 0;
        let mut time_player_count: usize = 0;

        for mut movable in ctx.db.movables().iter() {
            match movable.kind {
                EntityKind::Ball => { Self::despawn_ball(ctx, engine, &movable.name) },
                EntityKind::SpacePlayer => {
                    movable.position = ctx.get_player_position(space_player_count, Side::Space);
                    Self::reset_player(ctx, engine, movable, &game);
                    space_player_count += 1;
                },
                EntityKind::TimePlayer => {
                    movable.position = ctx.get_player_position(time_player_count, Side::Time);
                    Self::reset_player(ctx, engine, movable, &game);
                    time_player_count += 1;
                }
            }
        }

        ctx.add_random_balls_for_each_player();
        
        game.state = GameState::Waiting;
        ctx.db.games().id().update(game);

    }

    fn reset_player(ctx: &ReducerContext, engine: &mut PhysicsEngine, mut player: Movable, game: &Game) {
        let spawn_with_ball = ctx.are_players_spawning_with_ball();
        let player_name = player.name.clone();
        let player_position = player.position.clone();
        let player_state = match (game.has_started(), spawn_with_ball) {
            (false, _) => EntityState::Standby,
            (true, true) => EntityState::ReadyToThrow,
            (true, false) => EntityState::ReadyToMove
        };
        
        player.velocity = Velocity::ZERO;
        player.state = player_state.clone();
        engine.upsert_player(&player, PLAYER_DIMENSION, PLAYER_BOUNCINESS);
        ctx.db.movables().name().update(player);

        if player_state == EntityState::ReadyToThrow {
            Self::spawn_ball_for_player(ctx, engine, player_name, player_position);
        }
    }

    fn despawn_ball(ctx: &ReducerContext, engine: &mut PhysicsEngine, name: &str) {
        Self::despawn_movable(ctx, engine, name);
    }
    
    fn despawn_player(ctx: &ReducerContext, engine: &mut PhysicsEngine, name: &str) {
        Self::despawn_movable(ctx, engine, name);
        ctx.db.routables().name().delete(String::from(name));
    }

    fn despawn_movable(ctx: &ReducerContext, engine: &mut PhysicsEngine, name: &str) {
        if ctx.db.movables().name().delete(String::from(name)) {
            match engine.delete(&name) {
                Ok(maybe_related_entity_name) => {
                    if let Some(related_entity_name) = maybe_related_entity_name {
                        if let Some(mut related_entity) = ctx.find_movable(&related_entity_name) {
                            if related_entity.is_player(EntityState::ReadyToThrow) {
                                related_entity.state = EntityState::ReadyToMove;
                                ctx.db.movables().name().update(related_entity);
                            } else if related_entity.kind == EntityKind::Ball {
                                match engine.delete(&related_entity_name) {
                                    Ok(_) => { ctx.db.movables().delete(related_entity); },
                                    Err(message) => { log::error!("Could not remove ball when player {} despawned: {}", name, message) }
                                }
                            }
                        }
                    }
                },
                Err(message) => {
                    log::error!("Reference not found while despawning: {}", message);
                }
            }
        } else {
            log::error!("Movable not found while despawning: {}", name);
        }
    }

    fn spawn_ball_for_player(ctx: &ReducerContext, engine: &mut PhysicsEngine, player_name: String, player_position: Position) {
        let ball_name = ctx.add_ball(player_position, Velocity::ZERO);
        if let Some(ball) = ctx.find_movable(&ball_name) {
            Self::spawn_ball(ctx, engine, ball);
            match engine.catch_ball(&player_name, &ball_name) {
                Ok(()) => {},
                Err(message) => { log::error!("Could not spawn player {player_name} with ball in hand: {message}") }
            }
        }
    }
}
