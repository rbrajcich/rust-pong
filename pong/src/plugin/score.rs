//!
//! The pong score module contains the ScorePlugin, which keeps track of
//! the game score, as well as managing the on-screen components that display
//! info about the score and end of game results.
//! 

// -----------------------------------------------------------------------------
// Included Symbols

use bevy::prelude::*;
use bevy::sprite::Anchor;

use bevy_dyn_fontsize::{DynamicFontsizePlugin, DynamicFontSize};

use crate::common::*;

// -----------------------------------------------------------------------------
// Constants

const SCORE_FONT_SIZE_AS_SCREEN_PCT: f32 = 0.2;
const WIN_FONT_SIZE_AS_SCREEN_PCT: f32 = 0.04;
const PADDING_UNDER_SCORE_AS_SCREEN_PCT: f32 = 0.02;
const WINNING_SCORE: u8 = 10;

const P1_WIN_TEXT: &str = "Player 1 Wins!";
const P2_WIN_TEXT: &str = "Player 2 Wins!";

const SCORE_TEXT_Y: f32 = ARENA_HEIGHT / 2f32; // Top of arena in Y coords
const SCORE_TEXT_HEIGHT: f32 = SCORE_FONT_SIZE_AS_SCREEN_PCT * ARENA_HEIGHT;
const SCORE_BOTTOM: f32 = SCORE_TEXT_Y - SCORE_TEXT_HEIGHT;
const PADDING_UNDER_SCORE: f32 = PADDING_UNDER_SCORE_AS_SCREEN_PCT * ARENA_HEIGHT;
const WIN_TEXT_Y: f32 = SCORE_BOTTOM - PADDING_UNDER_SCORE;
const WIN_TEXT_HEIGHT: f32 = WIN_FONT_SIZE_AS_SCREEN_PCT * ARENA_HEIGHT;
const RIGHT_SIDE_CENTER_X: f32 = ARENA_WIDTH / 4f32;
const LEFT_SIDE_CENTER_X: f32 = -RIGHT_SIDE_CENTER_X;

// -----------------------------------------------------------------------------
// Public API

///
/// This plugin adds all score keeping functionality to the game. Note that it
/// does not detect score events on its own, or alter game state. It interacts
/// with other game logic to handle such things by sending or receiving
/// the events contained in this module.
///
/// This plugin will only work properly if the app contains a single Window
/// and a single Camera2d entity.
///
/// To ensure necessary ordering constraints are maintained, see descriptions
/// of below Events and SystemSets.
///
pub struct ScorePlugin;

impl Plugin for ScorePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(DynamicFontsizePlugin::default())
            .insert_resource(Score::default())
            .add_event::<PlayerScored>()
            .add_event::<MaxScoreReached>()
            .add_event::<ClearScores>()
            .add_systems(Startup, setup.in_set(Systems::Startup))
            .add_systems(Update,
                (handle_player_score, clear_scores).in_set(Systems::Update)
            );
    }
}

///
/// This event should be triggered by other code to notify the score module when
/// a player score has been detected (including the scorer's PlayerId).
///
#[derive(Event)]
pub struct PlayerScored(pub PlayerId);

///
/// This event will be triggered by the score module when a player has reached
/// the winning score and displayed the results. Other game logic should listen
/// for this event and move to an end-of-game state as well.
///
#[derive(Event)]
pub struct MaxScoreReached;

///
/// This event should be triggered by other code to notify the score module when
/// it should reset the scores to 0 and reflect this on-screen.
///
#[derive(Event)]
pub struct ClearScores;

///
/// Contains the SystemSets relevant to external code using this plugin.
/// These are exposed to enable proper ordering constraints in the game.
/// Code using this plugin should ensure the requirements are met for each:
///
/// The single in-game Camera2d MUST be created in the startup state, BEFORE
/// the Startup SystemSet here runs.
/// 
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum Systems {

    ///
    /// The required single Camera2d Entity MUST be created in the startup phase
    /// BEFORE this SystemSet is run.
    ///
    Startup,

    ///
    /// Systems writing PlayerScored or ClearScores Events SHOULD occur in the
    /// Update schedule BEFORE this SystemSet is run, to ensure score changes
    /// are reflected in the same game loop iteration they are detected.
    ///
    Update,
}

// -----------------------------------------------------------------------------
// Private Resources

// Resource to track the current score of each player
#[derive(Resource, Default)]
struct Score {
    p1: u8,
    p2: u8,
}

// -----------------------------------------------------------------------------
// Private Components

// Component for the ScoreText Entity of each player (on-screen score numbers)
#[derive(Component)]
struct ScoreText(PlayerId);

// Component for the WinText Entity of each player ("Player X Won!")
#[derive(Component)]
struct WinText(PlayerId);

// -----------------------------------------------------------------------------
// Private Systems

//
// Setup system to spawn each of the 4 on-screen Entities managed by the score
// module. Note that content and visibility of each may change, but they are
// all spawned during startup and exist throughout the duration of the game.
//
// The first 2 Entities are ScoreText - one on each side of the screen
// for each player. They each start at "0" and will count up each time the
// associated player scores. They will always be visible.
//
// The other 2 Entities are WinText - one on each side of the screen for each
// player. Each has appropriate text to announce when that player wins. The
// text of these will never change, but they both start hidden and will only
// be made visible once the associated player has won the game.
//
fn setup(
    mut commands: Commands,
    camera_entity: Single<Entity, With<Camera2d>>,
) {
    commands.spawn((
        ScoreText(Player1),
        DynamicFontSize { 
            height_in_world: SCORE_TEXT_HEIGHT,
            render_camera: camera_entity.entity(),
        },
        Text2d::new("0"),
        Anchor::TopCenter,
        Transform::from_translation(
            Vec3::new(LEFT_SIDE_CENTER_X, SCORE_TEXT_Y, Z_BEHIND_GAMEPLAY)
        ),
    ));

    commands.spawn((
        ScoreText(Player2),
        DynamicFontSize {
            height_in_world: SCORE_TEXT_HEIGHT,
            render_camera: camera_entity.entity(),
        },
        Text2d::new("0"),
        Anchor::TopCenter,
        Transform::from_translation(
            Vec3::new(RIGHT_SIDE_CENTER_X, SCORE_TEXT_Y, Z_BEHIND_GAMEPLAY)
        ),
    ));

    commands.spawn((
        WinText(Player1),
        DynamicFontSize {
            height_in_world: WIN_TEXT_HEIGHT,
            render_camera: camera_entity.entity(),
        },
        Text2d::new(P1_WIN_TEXT),
        Anchor::TopCenter,
        Transform::from_translation(
            Vec3::new(LEFT_SIDE_CENTER_X, WIN_TEXT_Y, Z_BEHIND_GAMEPLAY)
        ),
        Visibility::Hidden,
    ));

    commands.spawn((
        WinText(Player2),
        DynamicFontSize {
            height_in_world: WIN_TEXT_HEIGHT,
            render_camera: camera_entity.entity(),
        },
        Text2d::new(P2_WIN_TEXT),
        Anchor::TopCenter,
        Transform::from_translation(
            Vec3::new(RIGHT_SIDE_CENTER_X, WIN_TEXT_Y, Z_BEHIND_GAMEPLAY)
        ),
        Visibility::Hidden,
    ));
}

// 
// System to handle events generated when a player has scored. This system
// will update the score as needed (both internally and adjust entities).
// It will also check after each score received whether or not a player has
// won. If so, it will generate the MaxScoreReached event.
//
fn handle_player_score(
    mut events: EventReader<PlayerScored>,
    mut event_writer: EventWriter<MaxScoreReached>,
    mut scores: ResMut<Score>,
    score_texts: Query<(&mut Text2d, &ScoreText)>,
    win_texts: Query<(&mut Visibility, &WinText)>,
) {

    // Early return in case of no events
    if events.is_empty() {
        return;
    }

    let (p1_score_txt, p2_score_txt) = score_texts.into_iter()
        .map(|(text2d, score_text)| (score_text.0, text2d.into_inner()))
        .as_per_player();

    let (p1_win_txt, p2_win_txt) = win_texts.into_iter()
        .map(|(vis, win_text)| (win_text.0, vis.into_inner()))
        .as_per_player();

    // Handle each score event (realistically only one will have happened)
    for PlayerScored(scorer) in events.read() {

        // Add to score for applicable player
        match scorer {
            Player1 => {
                scores.p1 += 1;
                p1_score_txt.0 = scores.p1.to_string();
            },
            Player2 => {
                scores.p2 += 1;
                p2_score_txt.0 = scores.p2.to_string();
            },
        }

        // Detect if either player has won
        if scores.p1 >= WINNING_SCORE {
            event_writer.write(MaxScoreReached);
            *p1_win_txt = Visibility::Visible;
            break;
        } else if scores.p2 >= WINNING_SCORE {
            event_writer.write(MaxScoreReached);
            *p2_win_txt = Visibility::Visible;
            break;
        }
    }
}

// System to clear scores back to 0 and return UI elements to original states
fn clear_scores(
    mut events: EventReader<ClearScores>,
    mut scores: ResMut<Score>,
    score_texts: Query<&mut Text2d, With<ScoreText>>,
    win_texts: Query<&mut Visibility, With<WinText>>,
) {
    if !events.is_empty() {
        events.clear();

        *scores = Score { p1: 0, p2: 0 };

        for mut score_text in score_texts.into_iter() {
            score_text.0 = String::from("0");
        }

        for mut win_text in win_texts.into_iter() {
            *win_text = Visibility::Hidden;
        }
    }
}
