use spacetimedb::{ReducerContext, Table};
use crate::{ecs::{clock::clocks, game::{games, Game}, movable::movables, routable::routables, stat::stats, Clock, EntityKind, EntityState, GameState}, Helpers, GAME_AUTO_START_PAUSE_BETWEEN_GAMES, GAME_RESET_AT, GAME_WONT_RESTART, INFINITE_DURATION};
use super::{System, UpdateContext};

pub struct GameSystem;

impl System for GameSystem {
    fn handle(context: &mut UpdateContext, next: impl Fn(&mut UpdateContext)) {
        let game = match context.ctx.get_game() {
            Some(game) => game,
            None => return,
        };

        let clock = match context.ctx.get_clock() {
            Some(clock) => clock,
            None => return,
        };

        if should_tick(&game, &clock) {
            if game.has_started() {
                Self::stop_game_if_needed(context.ctx, context.step, game, clock);
            } else {
                if Self::start_game_if_needed(context.ctx, context.step, game, clock) {
                    return;
                }
            }
        }
        
        next(context);
    }
}

fn should_tick(game: &Game, clock: &Clock) -> bool {
    (game.has_started() || clock.remaning != 0.0) && clock.remaning != GAME_WONT_RESTART as f32
}

impl GameSystem {
    fn start_game_if_needed(ctx: &ReducerContext, step: f32, game: Game, mut clock: Clock) -> bool {
        clock.remaning += step;

        if clock.remaning >= 0.0 {
            Self::start_game(ctx, game, clock);
            return true;
        } else {
            Self::update_remaining_if_needed(ctx, game, &clock);
            ctx.db.clocks().id().update(clock);
        }

        false
    }

    fn stop_game_if_needed(ctx: &ReducerContext, step: f32, game: Game, mut clock: Clock) {
        if clock.remaning < INFINITE_DURATION as f32 {
            clock.remaning -= step;
        }

        if clock.remaning <= 0.0 {
            Self::stop_game(ctx, game, clock);
        } else {
            Self::update_remaining_if_needed(ctx, game, &clock);
            ctx.db.clocks().id().update(clock);
        }
    }

    fn update_remaining_if_needed(ctx: &ReducerContext, mut game: Game, clock: &Clock) {
        let remaining_seconds = if clock.remaning < 0.0 { clock.remaning.floor() } else { clock.remaning.ceil() } as i8;
        if game.remaining != remaining_seconds {
            game.remaining = remaining_seconds;
            if remaining_seconds == GAME_RESET_AT {
                game.state = GameState::Reset;
            }

            ctx.db.games().id().update(game);
        }
    }

    fn start_game(ctx: &ReducerContext, mut game: Game, mut clock: Clock) {
        const INITIAL_GAME_STATE: GameState = GameState::GameStarted;
        let game_duration = ctx.get_game_duration();
        game.state = INITIAL_GAME_STATE;
        game.space_score = 0;
        game.time_score = 0;
        game.mvp = String::from("");
        clock.remaning = game_duration as f32;
        game.remaining = game_duration as i8;

        ctx.db.clocks().id().update(clock);
        ctx.db.games().id().update(game);
        Self::update_players_state(ctx, &INITIAL_GAME_STATE);
    }

    fn stop_game(ctx: &ReducerContext, mut game: Game, mut clock: Clock) {
        let game_state = match game.space_score - game.time_score {
            n if n > 0 => GameState::GameEndedInSpaceWin,
            0 => GameState::GameEndedInTie,
            _ => GameState::GameEndedInTimeWin,
        };

        Self::update_players_state(ctx, &game_state);

        game.remaining = if ctx.is_game_auto_starting() { GAME_AUTO_START_PAUSE_BETWEEN_GAMES } else { GAME_WONT_RESTART };
        clock.remaning = game.remaining as f32;
        
        game.state = game_state;
        game.mvp = proclaim_mvp(ctx);

        ctx.db.clocks().id().update(clock);
        ctx.db.games().id().update(game);
    }

    fn update_players_state(ctx: &ReducerContext, game_state: &GameState) {
        for mut player in ctx.db.movables().iter().filter(|m| m.kind == EntityKind::SpacePlayer || m.kind == EntityKind::TimePlayer) {
            ctx.db.routables().name().delete(player.name.clone());

            player.state = match (game_state, &player.kind, &player.state) {
                (GameState::GameEndedInSpaceWin, EntityKind::SpacePlayer, _) |
                (GameState::GameEndedInTimeWin, EntityKind::TimePlayer, _) => EntityState::WonGame,
                (GameState::GameEndedInSpaceWin, EntityKind::TimePlayer, _) |
                (GameState::GameEndedInTimeWin, EntityKind::SpacePlayer, _) => EntityState::LostGame,
                (GameState::GameStarted, _, EntityState::Standby) => EntityState::ReadyToMove,
                _ => EntityState::Standby
            };

            ctx.db.movables().name().update(player);
        }
    }
}

fn proclaim_mvp(ctx: &ReducerContext) -> String {
    let mut mvp: String = String::from("");
    let mut record: f32 = 0.0;
    for mut stat in ctx.db.stats().iter() {
        if stat.points > record {
            record = stat.points;
            mvp = stat.name.clone();
        }

        stat.points = 0.0;
        ctx.db.stats().name().update(stat);
    }

    mvp
}