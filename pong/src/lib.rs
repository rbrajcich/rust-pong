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
const MIN_WINDOW_WIDTH: f32 = 160.0;
const MIN_WINDOW_HEIGHT: f32 = 90.0;
const MAX_WINDOW_WIDTH: f32 = 7680.0;
const MAX_WINDOW_HEIGHT: f32 = 4320.0;

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
                resize_constraints: WindowResizeConstraints {
                    min_width: MIN_WINDOW_WIDTH,
                    min_height: MIN_WINDOW_HEIGHT,
                    max_width: MAX_WINDOW_WIDTH,
                    max_height: MAX_WINDOW_HEIGHT,
                },
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
