use std::collections::HashMap;

use spacetimedb::{ReducerContext, Table};

use crate::{ecs::{movable::movables, EntityState, Movable, Position, Routable, RouteAction, Velocity}, services::PhysicsEngine, Helpers, DESTINATION_TOLERANCE, PLAYER_HIT_COUNTER, PLAYER_SPEED, PLAYER_STALL_COUNTER, PLAYER_STALL_DISTANCE, PLAYER_THROW_COUNTER};
use crate::ecs::components::routable::routables;

use super::{System, UpdateContext};

pub struct RoutingSystem;

impl System for RoutingSystem {
    fn handle(update_context: &mut UpdateContext, next: impl Fn(&mut UpdateContext)) {
        let mut players_moving: HashMap<String, Position> = HashMap::new();
        Self::route_movables(update_context.ctx, update_context.engine, &mut players_moving);
        next(update_context);
        Self::stop_players_who_collided_with_wall(update_context.ctx, &mut update_context.engine, &mut players_moving);
        Self::track_stalling_players(update_context.ctx, update_context.engine, &players_moving);
    }
}

impl RoutingSystem {
    fn route_movables(ctx: &ReducerContext, engine: &mut PhysicsEngine, players_moving: &mut HashMap<String, Position>) {
        for routable in ctx.db.routables().iter() {
            if let Some(movable) = ctx.find_movable(&routable.name) {
                match routable.action {
                    RouteAction::Move => Self::route_player(ctx, engine, players_moving, routable, movable),
                    RouteAction::Throw => Self::route_ball(ctx, engine, routable, movable),
                    RouteAction::Hit => Self::route_hit(ctx, routable, movable)
                }
            } else {
                log::warn!("Could not find matching movable for routable {}", routable.name);
                ctx.db.routables().delete(routable);
            }
        }
    }

    fn track_stalling_players(ctx: &spacetimedb::ReducerContext, _: &mut PhysicsEngine, players_moving: &HashMap<String, Position>) {
        for (name, initial_position) in players_moving.iter() {
            if let Some(movable) = ctx.find_movable(name) {
                if let Some(mut routable) = ctx.find_routable(name) {
                    let distance_travelled = initial_position.delta_norm(&movable.position);
                    if distance_travelled < PLAYER_STALL_DISTANCE {
                        routable.counter += 1;
                        ctx.db.routables().name().update(routable);
                    } else if routable.counter > 0 {
                        routable.counter = 0;
                        ctx.db.routables().name().update(routable);
                    }
                } else {
                    log::error!("Could not find routable for name {}", name);
                }
            }
        }
    }

    fn stop_players_who_collided_with_wall(ctx: &ReducerContext, engine: &mut PhysicsEngine, players_moving: &mut HashMap<String, Position>) {
        for collision in engine.get_player_wall_collision_events() {
            players_moving.remove(&collision.player_name);

            if let Some(routable) = ctx.find_routable(&collision.player_name) {
                if routable.action == RouteAction::Move {
                    ctx.db.routables().delete(routable);
                }
            }

            if let Some(mut movable) = ctx.find_movable(&collision.player_name) {
                if movable.state == EntityState::Moving {
                    movable.state = EntityState::ReadyToMove;
                    ctx.db.movables().name().update(movable);
                }
            }
        }
    }

    fn route_player(ctx: &ReducerContext, engine: &mut PhysicsEngine, players_moving: &mut HashMap<String, Position>, routable: Routable, mut player: Movable) {
        let mut should_update = false;
        match player.state {
            EntityState::Moving | EntityState::ReadyToMove => {
                let velocity = player.position.get_velocity_vector(&routable.destination, PLAYER_SPEED, DESTINATION_TOLERANCE);

                if player.state == EntityState::ReadyToMove {
                    player.state = EntityState::Moving;
                    should_update = true;
                }
    
                if velocity != Velocity::ZERO && routable.counter < PLAYER_STALL_COUNTER {
                    match engine.set_velocity(&routable.name, velocity) {
                        Ok(()) => {
                            players_moving.insert(player.name.clone(), player.position.clone());
                        },
                        Err(message) => log::error!("Could not route player {}: {}", &player.name, message)
                    }
                } else {
                    match engine.set_velocity(&routable.name, Velocity::ZERO) {
                        Ok(()) => {
                            ctx.db.routables().delete(routable);
                        },
                        Err(message) => log::error!("Could not stop stalled player {}: {}", &player.name, message)
                    }
                    player.state = EntityState::ReadyToMove;
                    should_update = true;
                }
            },
            EntityState::Hit => {
                ctx.db.routables().delete(routable);
            },
            _ => {
                match engine.set_velocity(&player.name, Velocity::ZERO) {
                    Ok(()) => {
                        ctx.db.routables().delete(routable);

                        if player.state == EntityState::Moving {
                            player.state = EntityState::ReadyToMove;
                            should_update = true;
                        }
    
                        if player.velocity != Velocity::ZERO {
                            player.velocity = Velocity::ZERO;
                            should_update = true;
                        }
                    },
                    Err(message) => log::error!("Could not stop movable {}: {}", &player.name, message)
                }
            }
        }

        if should_update {
            ctx.db.movables().name().update(player);
        }
    }

    fn route_ball(ctx: &ReducerContext, engine: &mut PhysicsEngine, mut routable: Routable, mut player: Movable) {
        match player.state {
            EntityState::ReadyToThrow => {
                match engine.apply_impulse(&player.name, Velocity { x: 0.0, y: 0.0, z: 200.0 }) {
                    Ok(()) => {
                        player.state = EntityState::Throwing;
                        ctx.db.movables().name().update(player);
                    },
                    Err(message) => { log::warn!("Could not apply impulse to throwing player {}: {}", player.name, message) }
                }
            },
            EntityState::Throwing => {
                routable.counter += 1;
                if routable.counter >= PLAYER_THROW_COUNTER {
                    let destination = routable.destination;
                    ctx.db.routables().delete(routable);
                    match engine.throw_ball(&player.name.clone(), player.kind.clone(), destination) {
                        Ok(()) => { },
                        Err(message) => { log::error!("Player {} could not throw ball: {}", player.name, message); }
                    }

                    player.state = EntityState::ReadyToMove;
                    ctx.db.movables().name().update(player);
                } else {
                    ctx.db.routables().name().update(routable);
                }
            },
            _ => {
                ctx.db.routables().delete(routable);
            }
        }
    }

    fn route_hit(ctx: &ReducerContext, mut routable: Routable, mut player: Movable) {
        if routable.counter >= PLAYER_HIT_COUNTER {
            ctx.db.routables().delete(routable);
            player.state = EntityState::ReadyToMove;
            ctx.db.movables().name().update(player);
        } else {
            routable.counter += 1;
            ctx.db.routables().name().update(routable);
        }
    }
}