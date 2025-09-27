//!
//! This module implements the Pong game in its entirety, as a Plugin, including
//! the game window, setup and all game logic.
//!

// -------------------------------------------------------------------------------------------------
// Module Declarations

mod arena;
mod ball;
mod common;
mod paddle;
mod score;

// -------------------------------------------------------------------------------------------------
// Included Symbols

use bevy::prelude::*;
use bevy::window::PresentMode;
use bevy::window::WindowResolution;

use arena::ArenaPlugin;
use ball::{BallOffScreen, BallPlugin, ResetBall, StartBall};
use common::*;
use paddle::PaddlePlugin;
use score::{ClearScores, MaxScoreReached, PlayerScored, ScorePlugin};

// -------------------------------------------------------------------------------------------------
// Constants

const PONG_WINDOW_TITLE: &str = "Rust Pong";
const INITIAL_WINDOW_WIDTH: f32 = 1600.0;
const INITIAL_WINDOW_HEIGHT: f32 = 900.0;
const WINDOW_SIZE_CONSTRAINTS: WindowResizeConstraints = WindowResizeConstraints {
    min_width: 160.0,
    min_height: 90.0,
    max_width: 7680.0,
    max_height: 4320.0,
};
const TIME_BEFORE_FIRST_ROUND_SECS: f32 = 2.0;
const TIME_BETWEEN_ROUNDS_SECS: f32 = 1.0;
const TIME_BETWEEN_GAMES_SECS: f32 = 3.0;

// -------------------------------------------------------------------------------------------------
// Public API

///
/// The actual plugin to add to a base Bevy app to run the rust pong game. This plugin
/// implements the Pong game in its entirety, including the game window,
/// entity setup, and all runtime game logic.
///
pub struct PongPlugin;

impl Plugin for PongPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: PONG_WINDOW_TITLE.to_string(),
                resize_constraints: WINDOW_SIZE_CONSTRAINTS,
                present_mode: PresentMode::AutoNoVsync,
                //mode: WindowMode::BorderlessFullscreen(MonitorSelection::Primary),
                resolution: WindowResolution::new(INITIAL_WINDOW_WIDTH, INITIAL_WINDOW_HEIGHT),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(ArenaPlugin)
        .add_plugins(BallPlugin)
        .add_plugins(PaddlePlugin)
        .add_plugins(ScorePlugin)
        .init_resource::<RoundStartTimer>()
        .init_resource::<IsBetweenGames>()
        .add_systems(PostStartup, start_first_round_timer)
        .add_systems(
            Update,
            (
                update_round_timer.before(score::Systems::ClearScoresRcvr),
                handle_ball_off_screen
                    .before(ball::Systems::ResetBallRcvr)
                    .before(score::Systems::PlayerScoredRcvr),
                handle_game_end,
            ),
        )
        .configure_sets(
            Startup,
            (arena::Systems::CameraSetup.before(score::Systems::SetupAfterCamera),),
        )
        .configure_sets(
            Update,
            (ball::Systems::BallOffScreenSndr.before(handle_ball_off_screen),),
        );
    }
}

// -------------------------------------------------------------------------------------------------
// Private Resources

// Timer which counts down to start of next round, when between rounds and/or games.
#[derive(Resource, Default)]
struct RoundStartTimer(Timer);

// Boolean state resource signifying if we are between games (true) or just rounds (false).
#[derive(Resource, Default)]
struct IsBetweenGames(bool);

// -------------------------------------------------------------------------------------------------
// Private Systems

// After everything is set up, start the timer for gameplay to begin
fn start_first_round_timer(mut round_timer: ResMut<RoundStartTimer>) {
    round_timer.0 = Timer::from_seconds(TIME_BEFORE_FIRST_ROUND_SECS, TimerMode::Once);
}

//
// System to handle expiring round timer (i.e. time to start a round).
// Should start the ball moving and if it's a new game, clear the scoreboard.
//
fn update_round_timer(
    time: Res<Time>,
    mut round_timer: ResMut<RoundStartTimer>,
    mut between_games: ResMut<IsBetweenGames>,
    mut clear_score_events: EventWriter<ClearScores>,
    mut start_ball_events: EventWriter<StartBall>,
) {
    round_timer.0.tick(time.delta());
    if round_timer.0.just_finished() {
        // Reset for new game if needed
        if between_games.0 {
            between_games.0 = false;
            clear_score_events.write(ClearScores);
        }

        // Start round
        start_ball_events.write(StartBall);
    }
}

//
// System to handle ball off screen events from ball plugin, and trigger associated
// actions to reset the ball, increment score, and start the timer until the next round.
//
fn handle_ball_off_screen(
    mut event_reader: EventReader<BallOffScreen>,
    mut score_events: EventWriter<PlayerScored>,
    mut reset_events: EventWriter<ResetBall>,
    mut round_timer: ResMut<RoundStartTimer>,
) {
    if let Some(event) = event_reader.read().next() {
        score_events.write(PlayerScored(match event {
            BallOffScreen::Left => Player2,
            BallOffScreen::Right => Player1,
        }));
        reset_events.write(ResetBall);
        round_timer.0 = Timer::from_seconds(TIME_BETWEEN_ROUNDS_SECS, TimerMode::Once);
        event_reader.clear();
    }
}

//
// System to handle 'end of game' scenario when a player has reached the winning score.
// Essentially just note we are between games and extend the between-round timer duration.
//
fn handle_game_end(
    mut events: EventReader<MaxScoreReached>,
    mut round_timer: ResMut<RoundStartTimer>,
    mut between_games: ResMut<IsBetweenGames>,
) {
    if !events.is_empty() {
        events.clear();
        between_games.0 = true;
        round_timer.0 = Timer::from_seconds(TIME_BETWEEN_GAMES_SECS, TimerMode::Once);
    }
}

// -------------------------------------------------------------------------------------------------
// Unit Tests

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_start_timer_system() {
        let mut world = World::default();

        // Prep resource that system will affect
        world.init_resource::<RoundStartTimer>();

        // Run the system
        let timer_sys = world.register_system(start_first_round_timer);
        world.run_system(timer_sys).unwrap();

        // Validate that timer has been started correctly
        let timer = world.get_resource::<RoundStartTimer>().unwrap();
        assert_eq!(
            timer.0.remaining(),
            Duration::from_secs_f32(TIME_BEFORE_FIRST_ROUND_SECS),
            "Expected initial time of {} but got {}",
            TIME_BEFORE_FIRST_ROUND_SECS,
            timer.0.remaining().as_secs_f32(),
        );
        assert!(!timer.0.paused(), "Expected timer to be unpaused");
        assert_eq!(timer.0.mode(), TimerMode::Once, "Expected TimerMode::Once");
    }

    #[test]
    fn test_update_timer_sys_no_trigger() {
        test_update_timer_sys_helper(&UpdateTimerSysHelperCfg {
            timer_expires: false,
            between_games_before: false,
            exp_score_clear: false,
            exp_start_ball: false,
            exp_between_games_after: false,
        });
    }

    #[test]
    fn test_update_timer_sys_w_trigger() {
        test_update_timer_sys_helper(&UpdateTimerSysHelperCfg {
            timer_expires: true,
            between_games_before: false,
            exp_score_clear: false,
            exp_start_ball: true,
            exp_between_games_after: false,
        });
    }

    #[test]
    fn test_update_timer_sys_between_no_trigger() {
        test_update_timer_sys_helper(&UpdateTimerSysHelperCfg {
            timer_expires: false,
            between_games_before: true,
            exp_score_clear: false,
            exp_start_ball: false,
            exp_between_games_after: true,
        });
    }

    #[test]
    fn test_update_timer_sys_between_w_trigger() {
        test_update_timer_sys_helper(&UpdateTimerSysHelperCfg {
            timer_expires: true,
            between_games_before: true,
            exp_score_clear: true,
            exp_start_ball: true,
            exp_between_games_after: false,
        });
    }

    #[test]
    fn test_ball_off_screen_left() {
        test_ball_off_screen_sys_helper(&BallOffScreenSysHelperCfg {
            input_events: &[BallOffScreen::Left],
            exp_player_score: Some(PlayerScored(Player2)),
            exp_reset_ball: true,
            exp_timer_started: true,
        });
    }

    #[test]
    fn test_ball_off_screen_right() {
        test_ball_off_screen_sys_helper(&BallOffScreenSysHelperCfg {
            input_events: &[BallOffScreen::Right],
            exp_player_score: Some(PlayerScored(Player1)),
            exp_reset_ball: true,
            exp_timer_started: true,
        });
    }

    #[test]
    fn test_ball_off_screen_multi() {
        test_ball_off_screen_sys_helper(&BallOffScreenSysHelperCfg {
            input_events: &[
                BallOffScreen::Right,
                BallOffScreen::Right,
                BallOffScreen::Left,
            ],
            exp_player_score: Some(PlayerScored(Player1)),
            exp_reset_ball: true,
            exp_timer_started: true,
        });
    }

    #[test]
    fn test_ball_off_screen_no_input() {
        test_ball_off_screen_sys_helper(&BallOffScreenSysHelperCfg {
            input_events: &[],
            exp_player_score: None,
            exp_reset_ball: false,
            exp_timer_started: false,
        });
    }

    #[test]
    fn test_game_end_system() {
        let mut world = World::default();

        // Get our resources in place to run the system
        let mut max_score_events = Events::<MaxScoreReached>::default();
        max_score_events.send(MaxScoreReached);
        world.insert_resource(max_score_events);
        world.insert_resource(IsBetweenGames(false));
        world.init_resource::<RoundStartTimer>();

        // Run the system
        let game_end_sys = world.register_system(handle_game_end);
        world.run_system(game_end_sys).unwrap();

        // Validate IsBetweenGames state afterwards
        let is_between_games = world.get_resource::<IsBetweenGames>().unwrap();
        assert!(
            is_between_games.0,
            "Expected IsBetweenGames=true but it was false"
        );

        // Validate Timer was set as expected
        let round_timer = world.get_resource::<RoundStartTimer>().unwrap();
        assert_eq!(
            round_timer.0,
            Timer::from_seconds(TIME_BETWEEN_GAMES_SECS, TimerMode::Once),
            "Expected timer {:?} but got timer {:?}",
            Timer::from_seconds(TIME_BETWEEN_GAMES_SECS, TimerMode::Once),
            round_timer.0,
        );
    }

    // --- Helper Types ---

    struct UpdateTimerSysHelperCfg {
        timer_expires: bool,
        between_games_before: bool,
        exp_score_clear: bool,
        exp_start_ball: bool,
        exp_between_games_after: bool,
    }

    struct BallOffScreenSysHelperCfg<'a> {
        input_events: &'a [BallOffScreen],
        exp_player_score: Option<PlayerScored>,
        exp_reset_ball: bool,
        exp_timer_started: bool,
    }

    // --- Helper Functions ---

    fn test_update_timer_sys_helper(cfg: &UpdateTimerSysHelperCfg) {
        let mut world = World::default();

        // Get our resources in place based on the config given
        let mut time = Time::<()>::default();
        time.advance_by(if cfg.timer_expires {
            Duration::from_millis(1000)
        } else {
            Duration::from_millis(500)
        });
        world.insert_resource(time);
        world.insert_resource(IsBetweenGames(cfg.between_games_before));
        world.init_resource::<Events<ClearScores>>();
        world.init_resource::<Events<StartBall>>();
        world.insert_resource(RoundStartTimer(Timer::from_seconds(1f32, TimerMode::Once)));

        // Run the system
        let update_sys = world.register_system(update_round_timer);
        world.run_system(update_sys).unwrap();

        // Validate ClearScores events
        let clear_events = world.get_resource::<Events<ClearScores>>().unwrap();
        if cfg.exp_score_clear {
            assert!(
                !clear_events.is_empty(),
                "Expected a ClearScores event but got none"
            );
        } else {
            assert!(
                clear_events.is_empty(),
                "Expected no ClearScores but got one"
            );
        }

        // Validate StartBall events
        let start_events = world.get_resource::<Events<StartBall>>().unwrap();
        if cfg.exp_start_ball {
            assert!(
                !start_events.is_empty(),
                "Expected one StartBall event but got none"
            );
        } else {
            assert!(start_events.is_empty(), "Expected no StartBall but got one");
        }

        // Validate IsBetweenGames state afterwards
        let is_between_games = world.get_resource::<IsBetweenGames>().unwrap();
        if cfg.exp_between_games_after {
            assert!(
                is_between_games.0,
                "Expected IsBetweenGames=true but it was false"
            );
        } else {
            assert!(
                !is_between_games.0,
                "Expected IsBetweenGames=false but is was true"
            );
        }
    }

    fn test_ball_off_screen_sys_helper(cfg: &BallOffScreenSysHelperCfg) {
        let mut world = World::default();

        // Get our resources in place based on the config given
        let mut input_events = Events::<BallOffScreen>::default();
        for input_event in cfg.input_events {
            input_events.send(*input_event);
        }
        world.insert_resource(input_events);
        world.init_resource::<Events<PlayerScored>>();
        world.init_resource::<Events<ResetBall>>();
        world.init_resource::<RoundStartTimer>();

        // Run the system
        let ball_sys = world.register_system(handle_ball_off_screen);
        world.run_system(ball_sys).unwrap();

        // Validate expected PlayerScored event
        let score_events = world.get_resource::<Events<PlayerScored>>().unwrap();
        let mut event_cursor = score_events.get_cursor();
        let mut event_iter = event_cursor.read(score_events);
        if let Some(exp_score_event) = &cfg.exp_player_score {
            let score_event = event_iter
                .next()
                .expect("Expected a PlayerScored event but got none");
            assert_eq!(
                *score_event, *exp_score_event,
                "Expected event {:?} but got {:?}",
                *exp_score_event, *score_event,
            );
            assert!(
                event_iter.next().is_none(),
                "Expected one PlayerScored event but got more"
            );
        } else {
            assert!(
                event_iter.next().is_none(),
                "Expected no PlayerScored events but got one"
            );
        }

        // Validate ResetBall events
        let reset_events = world.get_resource_mut::<Events<ResetBall>>().unwrap();
        if cfg.exp_reset_ball {
            assert!(
                !reset_events.is_empty(),
                "Expected one ResetBall event but got none"
            );
        } else {
            assert!(
                reset_events.is_empty(),
                "Expected no ResetBall events but got one"
            );
        }

        // Validate Timer was started if expected
        let round_timer = world.get_resource::<RoundStartTimer>().unwrap();
        if cfg.exp_timer_started {
            assert!(
                !round_timer.0.paused(),
                "Expected RoundStartTimer to be running",
            );
            assert_eq!(
                round_timer.0.remaining().as_secs_f32(),
                TIME_BETWEEN_ROUNDS_SECS,
                "Expected timer set for {} secs but it was set for {}",
                TIME_BETWEEN_ROUNDS_SECS,
                round_timer.0.remaining().as_secs_f32(),
            );
        } else {
            assert_eq!(
                round_timer.0,
                Timer::default(),
                "Did not expect RoundStartTimer to have been started",
            );
        }
    }
}
