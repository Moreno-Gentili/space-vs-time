use spacetimedb::{ReducerContext, Table};

use crate::{ecs::movable::movables, services::PhysicsEngine};

use super::{System, UpdateContext};

pub struct MovementSystem;

impl System for MovementSystem {

    fn handle(update_context: &mut UpdateContext, next: impl Fn(&mut UpdateContext)) {
        Self::preview(update_context.ctx, update_context.engine, update_context.step);
        next(update_context);
    }
}

impl MovementSystem {
    fn preview(ctx: &ReducerContext, engine: &mut PhysicsEngine, step: f32) {
        
        engine.step(step);
    
        for mut movable in ctx.db.movables().iter() {
            match engine.update_movable(&mut movable) {
                Ok(true) => {
                    ctx.db.movables().name().update(movable);
                }
                Ok(false) => { }
                Err(err) => {
                    log::error!("Error while updating Movable: {}", err)
                }
            }
        }
    }
}