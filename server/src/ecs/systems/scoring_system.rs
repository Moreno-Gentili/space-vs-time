use std::collections::HashSet;
use spacetimedb::{rand::Rng, ReducerContext, Table};
use crate::{ecs::{game::games, movable::movables, routable::routables, stat::stats, EntityKind, EntityState, GameState, Movable, Position, Routable, RouteAction, Velocity}, services::{physics_engine::PlayerBallCollision, PhysicsEngine}, Helpers, HIT_IMPULSE_MULTIPLIER, SURE_CATCH_RELATIVE_SPEED, SURE_HIT_RELATIVE_SPEED };
use super::{System, UpdateContext};

pub struct ScoringSystem;

impl System for ScoringSystem {
    fn handle(context: &mut UpdateContext, next: impl Fn(&mut UpdateContext)) {
        next(context);
        Self::handle_ball_wall_collisions(context.engine);
        Self::handle_ball_player_collisions(context.ctx, context.engine);
    }
}

impl ScoringSystem {
    fn handle_ball_wall_collisions(engine: &mut PhysicsEngine) {
        let collisions = engine.get_ball_wall_collision_events().clone();
        for collision in collisions {
            match engine.reset_ball_colliders(&collision.ball_name) {
                Err(message) => log::error!("Could not reset colliders for ball {}: {}", &collision.ball_name, message),
                _ => { }
            }
        }
    }

    fn handle_ball_player_collisions(ctx: &ReducerContext, engine: &mut PhysicsEngine) {
        let collisions = engine.get_player_ball_collision_events().clone();
        let mut handled_balls: HashSet<String> = HashSet::new();
        for collision in collisions {
            if let Some(player) = ctx.find_movable(&collision.player_name) {
                if !&player.can_be_hit() {
                    continue;
                }

                if let Some(ball) = ctx.find_movable(&collision.ball_name) {
                    let ball_velocity = Velocity { z: 0.0, ..collision.velocity };
                    let relative_speed = player.velocity.delta_norm(&ball_velocity);
                    
                    Self::handle_ball_collision(ctx, engine, &collision, ball_velocity, relative_speed, player, ball, &mut handled_balls);
                } else {
                    log::error!("Could not find the movable for ball {} who collided with player {}", &collision.ball_name, &collision.player_name);
                }
            } else {
                log::error!("Could not find the movable for player {} who collided with ball {}", &collision.player_name, &collision.ball_name);
            }
        }
    }

    fn determine_impact_result(ctx: &ReducerContext, relative_speed: f32, player_state: &EntityState) -> ImpactResult {
        const RANGE: f32 = SURE_HIT_RELATIVE_SPEED - SURE_CATCH_RELATIVE_SPEED;
        let normalized = relative_speed - SURE_CATCH_RELATIVE_SPEED;

        match player_state {
            EntityState::Moving | EntityState::ReadyToMove | EntityState::Standby => {
                match normalized {
                    f32::MIN..=0.0 => ImpactResult::Catch,
                    RANGE..=f32::MAX => ImpactResult::Hit,
                    _ => match ctx.rng().gen_range(0.0..=1.0) >= (normalized / RANGE) {
                        true => ImpactResult::Catch,
                        false => ImpactResult::Hit
                    }
                }
            },
            _ => ImpactResult::Hit
        }
    }

    fn catch(ctx: &ReducerContext, engine: &mut PhysicsEngine, mut player: Movable, mut ball: Movable, ball_velocity: Velocity) {
        let impulse = Self::calculate_impulse(ball_velocity, 0.4);
        match engine.apply_impulse(&player.name, impulse) {
            Ok(()) => {
                match engine.catch_ball(&player.name, &ball.name) {
                    Ok(()) => {
                        player.state = EntityState::ReadyToThrow;
                        ctx.db.movables().name().update(player);

                        ball.state = EntityState::ReadyToThrow;
                        ctx.db.movables().name().update(ball);
                    },
                    Err(message) => { log::error!("Could not make the player catch the ball: {}", message); }
                }
            },
            Err(message) => { log::error!("Could not apply impulse to player on catch: {}", message); }
        }
    }

    fn hit(ctx: &ReducerContext, engine: &mut PhysicsEngine, mut player: Movable, ball: Movable, ball_velocity: Velocity, scorer: &Option<String>) {
        let impulse = Self::calculate_impulse(ball_velocity, 1.0);
        let player_name = player.name.clone();

        make_player_drop_ball_if_holding_one(ctx, engine, &player);

        match engine.apply_impulse(&player.name, impulse) {
            Ok(()) => {
                if let Some(mut game) = ctx.get_game() {
                    if player.kind == EntityKind::SpacePlayer {
                        game.time_score += 1;
                        game.state = GameState::TimeScored;
                    } else {
                        game.space_score += 1;
                        game.state = GameState::SpaceScored;
                    }

                    ctx.db.games().id().update(game);

                    let player_position = player.position.clone();

                    player.state = EntityState::Hit;
                    ctx.db.movables().name().update(player);

                    if let Some(previous_routable) = ctx.find_routable(&player_name) {
                        ctx.db.routables().delete(previous_routable);
                    }

                    ctx.db.routables().insert(Routable {
                        name: player_name.clone(),
                        counter: 0,
                        destination: player_position,
                        action: RouteAction::Hit,
                    });

                    match engine.reset_ball_colliders(&ball.name) {
                        Err(message) => log::error!("Could not reset colliders for ball {}: {}", &ball.name, message),
                        _ => { }
                    }

                    let rebound_impulse = Self::calculate_ball_rebound_impulse(ctx, ball.velocity);
                    match engine.apply_impulse(&ball.name, rebound_impulse) {
                        Ok(()) => {},
                        Err(message) => { log::error!("Could not make the ball rebound on impact with player {}: {}", &player_name, message); }
                    }

                    if let Some(scorer_name) = scorer {
                        Self::score_point_for(ctx, &scorer_name);
                    }

                    Self::detract_point_from(ctx, &player_name);
                }
            },
            Err(message) => { log::error!("Could not apply impulse to player on hit: {}", message); }
        }
    }

    fn bump(engine: &mut PhysicsEngine, player: Movable, ball_velocity: Velocity) {
        let impulse = Self::calculate_impulse(ball_velocity, 0.5);
        match engine.apply_impulse(&player.name, impulse) {
            Ok(()) => {
                // When a ball bumps into a player without scoring a point (it happens when it bounced), do not alter its state
            },
            Err(message) => { log::error!("Could not apply impulse to player on bump: {}", message); }
        }
    }

    fn calculate_impulse(ball_velocity: Velocity, intensity: f32) -> Velocity {
        Velocity {
            x: ball_velocity.x * HIT_IMPULSE_MULTIPLIER * intensity,
            y: ball_velocity.y * HIT_IMPULSE_MULTIPLIER * intensity,
            z: 0.0
        }
    }

    fn handle_ball_collision(ctx: &ReducerContext, engine: &mut PhysicsEngine, collision: &PlayerBallCollision, ball_velocity: Velocity, relative_speed: f32, player: Movable, ball: Movable, handled_balls: &mut HashSet<String>) {
        let ball_was_handled = handled_balls.contains(&collision.ball_name);
        let impact_result = Self::determine_impact_result(ctx, relative_speed, &player.state);
        match (impact_result, ball_was_handled, collision.ball_has_rebounded) {
            (ImpactResult::Catch, false, _) => {
                Self::catch(ctx, engine, player, ball, ball_velocity);
                handled_balls.insert(collision.ball_name.clone());
            },
            (ImpactResult::Hit, false, false) => {
                let scorer = &collision.tossed_by_name;
                Self::hit(ctx, engine, player, ball, ball_velocity, scorer);
                handled_balls.insert(collision.ball_name.clone());
            },
            _ => {
                Self::bump(engine, player, ball_velocity);
            }
        };
    }

    fn score_point_for(ctx: &ReducerContext, scorer_name: &str) {
        if let Some(mut stat) = ctx.find_stat(scorer_name) {
            stat.points += 1.0;
            ctx.db.stats().name().update(stat);
        }
    }

    fn detract_point_from(ctx: &ReducerContext, player_name: &str) {
        if let Some(mut stat) = ctx.find_stat(player_name) {
            stat.points -= 0.5;
            ctx.db.stats().name().update(stat);
        }
    }
    
    fn calculate_ball_rebound_impulse(ctx: &ReducerContext, impulse: Velocity) -> Velocity {
        Velocity {
            x: -impulse.x * ctx.rng().gen_range(1.0..1.2),
            y: -impulse.y * ctx.rng().gen_range(1.0..1.2),
            z: ctx.rng().gen_range(5.0..10.0)
        }
    }
}

fn make_player_drop_ball_if_holding_one(ctx: &ReducerContext, engine: &mut PhysicsEngine, player: &Movable) {
    let factor = if player.kind == EntityKind::SpacePlayer { 1.0 } else { -1.0 };
    let x_movement = ctx.rng().gen_range(0.3..1.0) * factor;
    match engine.throw_ball(&player.name, player.kind.clone(), Position {
        x: player.position.x + x_movement,
        ..player.position
    }) {
        Ok(()) => { },
        Err(_) => { }
    }
}

enum ImpactResult {
    Hit,
    Catch
}