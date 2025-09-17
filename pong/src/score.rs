//!
//! The pong score module contains the ScorePlugin, which keeps track of
//! the game score, as well as managing the on-screen components that display
//! info about the score and end of game results.
//!

// -------------------------------------------------------------------------------------------------
// Included Symbols

use bevy::prelude::*;
use bevy::sprite::Anchor;

use bevy_dyn_fontsize::{DynamicFontSize, DynamicFontsizePlugin};

use crate::common::*;

// -------------------------------------------------------------------------------------------------
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

// -------------------------------------------------------------------------------------------------
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
            .add_systems(
                Update,
                (handle_player_score, clear_scores).in_set(Systems::Update),
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

// -------------------------------------------------------------------------------------------------
// Private Resources

// Resource to track the current score of each player
#[derive(Resource, Default, Debug, PartialEq, Eq)]
struct Score {
    p1: u8,
    p2: u8,
}

// -------------------------------------------------------------------------------------------------
// Private Components

// Component for the ScoreText Entity of each player (on-screen score numbers)
#[derive(Component)]
struct ScoreText(PlayerId);

// Component for the WinText Entity of each player ("Player X Won!")
#[derive(Component)]
struct WinText(PlayerId);

// -------------------------------------------------------------------------------------------------
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
fn setup(mut commands: Commands, camera_entity: Single<Entity, With<Camera2d>>) {
    commands.spawn((
        ScoreText(Player1),
        DynamicFontSize {
            height_in_world: SCORE_TEXT_HEIGHT,
            render_camera: camera_entity.entity(),
        },
        Text2d::new("0"),
        Anchor::TopCenter,
        Transform::from_translation(Vec3::new(
            LEFT_SIDE_CENTER_X,
            SCORE_TEXT_Y,
            Z_BEHIND_GAMEPLAY,
        )),
    ));

    commands.spawn((
        ScoreText(Player2),
        DynamicFontSize {
            height_in_world: SCORE_TEXT_HEIGHT,
            render_camera: camera_entity.entity(),
        },
        Text2d::new("0"),
        Anchor::TopCenter,
        Transform::from_translation(Vec3::new(
            RIGHT_SIDE_CENTER_X,
            SCORE_TEXT_Y,
            Z_BEHIND_GAMEPLAY,
        )),
    ));

    commands.spawn((
        WinText(Player1),
        DynamicFontSize {
            height_in_world: WIN_TEXT_HEIGHT,
            render_camera: camera_entity.entity(),
        },
        Text2d::new(P1_WIN_TEXT),
        Anchor::TopCenter,
        Transform::from_translation(Vec3::new(LEFT_SIDE_CENTER_X, WIN_TEXT_Y, Z_BEHIND_GAMEPLAY)),
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
        Transform::from_translation(Vec3::new(
            RIGHT_SIDE_CENTER_X,
            WIN_TEXT_Y,
            Z_BEHIND_GAMEPLAY,
        )),
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

    let (p1_score_txt, p2_score_txt) = score_texts
        .into_iter()
        .map(|(text2d, score_text)| (score_text.0, text2d.into_inner()))
        .as_per_player();

    let (p1_win_txt, p2_win_txt) = win_texts
        .into_iter()
        .map(|(vis, win_text)| (win_text.0, vis.into_inner()))
        .as_per_player();

    // Handle each score event (realistically only one will have happened)
    for PlayerScored(scorer) in events.read() {
        // Add to score for applicable player
        match scorer {
            Player1 => {
                scores.p1 += 1;
                p1_score_txt.0 = scores.p1.to_string();
            }
            Player2 => {
                scores.p2 += 1;
                p2_score_txt.0 = scores.p2.to_string();
            }
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

// -------------------------------------------------------------------------------------------------
// Unit Tests

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_test_helpers::prelude::*;

    #[test]
    fn test_plugin_build() {
        let mut app = App::new();
        app.add_plugins(ScorePlugin);

        // Validate expected dependent plugin made it into the app
        assert!(
            app.is_plugin_added::<DynamicFontsizePlugin>(),
            "Expected DynamicFontsizePlugin to be added by ScorePlugin"
        );

        // Validate resources (including events) were added to the world
        let world = app.world();
        assert!(
            world.is_resource_added::<Score>(),
            "Expected Score resource to be added by ScorePlugin"
        );
        assert!(
            world.is_resource_added::<Events<PlayerScored>>(),
            "Expected PlayerScored event to be added by ScorePlugin"
        );
        assert!(
            world.is_resource_added::<Events<MaxScoreReached>>(),
            "Expected MaxScoreReached event to be added by ScorePlugin"
        );
        assert!(
            world.is_resource_added::<Events<ClearScores>>(),
            "Expected ClearScores event to be added by ScorePlugin"
        );
    }

    #[test]
    fn test_plugin_sys_added_setup() {
        validate_sys_in_plugin(ScorePlugin, Startup, setup, Some(Systems::Startup));
    }

    #[test]
    fn test_plugin_sys_added_handle_player_score() {
        validate_sys_in_plugin(
            ScorePlugin,
            Update,
            handle_player_score,
            Some(Systems::Update),
        );
    }

    #[test]
    fn test_plugin_sys_added_clear_scores() {
        validate_sys_in_plugin(ScorePlugin, Update, clear_scores, Some(Systems::Update));
    }

    #[test]
    fn test_event_cleanup() {
        let mut app = App::new();
        let world = app.add_plugins(ScorePlugin).world_mut();

        world.send_event(PlayerScored(Player1));
        world.send_event(MaxScoreReached);
        world.send_event(ClearScores);

        // One game loop should not wipe out event (due to double buffering)
        world.run_schedule(First);
        assert!(
            !world
                .get_resource::<Events<PlayerScored>>()
                .expect("Expected to find Events resource for PlayerScored")
                .is_empty(),
            "Expected PlayerScored event to still exist after one game loop",
        );
        assert!(
            !world
                .get_resource::<Events<MaxScoreReached>>()
                .expect("Expected to find Events resource for MaxScoreReached")
                .is_empty(),
            "Expected MaxScoreReached event to still exist after one game loop",
        );
        assert!(
            !world
                .get_resource::<Events<ClearScores>>()
                .expect("Expected to find Events resource for ClearScores")
                .is_empty(),
            "Expected ClearScores event to still exist after one game loop",
        );

        // After second game loop, events should be cleaned up automatically
        world.run_schedule(First);
        assert!(
            world
                .get_resource::<Events<PlayerScored>>()
                .expect("Expected to find Events resource for PlayerScored")
                .is_empty(),
            "Expected PlayerScored event to be cleared after two game loops",
        );
        assert!(
            world
                .get_resource::<Events<MaxScoreReached>>()
                .expect("Expected to find Events resource for MaxScoreReached")
                .is_empty(),
            "Expected MaxScoreReached event to be cleared after two game loops",
        );
        assert!(
            world
                .get_resource::<Events<ClearScores>>()
                .expect("Expected to find Events resource for ClearScores")
                .is_empty(),
            "Expected ClearScores event to be cleared after two game loops",
        );
    }

    #[test]
    fn test_setup_system() {
        let mut world = World::default();

        // Set up a system to create the Camera2d we'll need, plus the setup system itself
        let cam_create_sys =
            world.register_system(|mut commands: Commands| commands.spawn(Camera2d).id());
        let setup_sys = world.register_system(setup);

        // Run the systems
        let cam_entity = world.run_system(cam_create_sys).unwrap();
        world
            .run_system(setup_sys)
            .expect("Expected setup system to run successfully");

        // Get the ScoreText entities created by the setup system
        let mut query = world.query::<(&ScoreText, &DynamicFontSize, &Text2d)>();
        let mut query_iter = query.iter(&world);
        let first = query_iter
            .next()
            .expect("Expected to get 2 ScoreTexts from setup. Got none");
        let second = query_iter
            .next()
            .expect("Expected to get 2 ScoreTexts from setup. Got 1");
        assert!(
            query_iter.next().is_none(),
            "Expected to get 2 ScoreTexts from setup. Got more"
        );

        // Assert a few key items on each to validate proper creation
        assert_ne!(
            first.0.0, second.0.0,
            "Expected ScoreTexts to have unique PlayerId's"
        );
        for (_, dyn_font, text2d) in [first, second] {
            assert_eq!(
                dyn_font.render_camera, cam_entity,
                "Expected ScoreText to use Camera2d as render_camera entity"
            );
            assert_eq!(
                text2d.0, "0",
                "Expected ScoreTexts to start with '0' as text value"
            );
        }

        // Get the WinText entities created by the setup system
        let mut query = world.query::<(&WinText, &DynamicFontSize, &Visibility)>();
        let mut query_iter = query.iter(&world);
        let first = query_iter
            .next()
            .expect("Expected to get 2 WinTexts from setup. Got none");
        let second = query_iter
            .next()
            .expect("Expected to get 2 WinTexts from setup. Got 1");
        assert!(
            query_iter.next().is_none(),
            "Expected to get 2 WinTexts from setup. Got more"
        );

        // Assert a few key items on each to validate proper creation
        assert_ne!(
            first.0.0, second.0.0,
            "Expected WinTexts to have unique PlayerId's"
        );
        for (_, dyn_font, vis) in [first, second] {
            assert_eq!(
                dyn_font.render_camera, cam_entity,
                "Expected WinText to use Camera2d as render_camera entity"
            );
            assert_eq!(
                vis,
                Visibility::Hidden,
                "Expected WinTexts to start as hidden"
            );
        }
    }

    #[test]
    fn test_handle_player_score_system() {
        // Create world with necessary resources
        let mut world = World::default();
        world.init_resource::<Events<PlayerScored>>();
        world.init_resource::<Events<MaxScoreReached>>();
        world.init_resource::<Score>();

        // Systems we'll need for this test
        let cam_create_sys = world.register_system(
            // Create a camera (for setup sys)
            |mut commands: Commands| {
                commands.spawn(Camera2d);
            },
        );
        let setup_sys = world.register_system(setup); // Setup text entities in the world
        let score_sys = world.register_system(handle_player_score);

        // Prime the world by running our setup systems
        world.run_system(cam_create_sys).unwrap();
        world.run_system(setup_sys).unwrap();

        // Run system the first time with no event. Expect no change
        world.run_system(score_sys).unwrap();
        validate_scores(
            &mut world,
            0,
            0,
            "0",
            "0",
            false,
            false,
            "after run with no score events",
        );
        assert!(
            world
                .get_resource::<Events<MaxScoreReached>>()
                .unwrap()
                .is_empty(),
            "Expected 0 MaxScoreReached events after run with no score events",
        );

        // Run system again with a p1 score event. Expect p1 score increment
        world.send_event(PlayerScored(Player1));
        world.run_system(score_sys).unwrap();
        validate_scores(
            &mut world,
            1,
            0,
            "1",
            "0",
            false,
            false,
            "after run with p1 score event",
        );
        assert!(
            world
                .get_resource::<Events<MaxScoreReached>>()
                .unwrap()
                .is_empty(),
            "Expected 0 MaxScoreReached events after run with p1 score event",
        );

        // Run system again with a p2 score event. Expect p2 score increment
        world.send_event(PlayerScored(Player2));
        world.run_system(score_sys).unwrap();
        validate_scores(
            &mut world,
            1,
            1,
            "1",
            "1",
            false,
            false,
            "after run with p2 score event",
        );
        assert!(
            world
                .get_resource::<Events<MaxScoreReached>>()
                .unwrap()
                .is_empty(),
            "Expected 0 MaxScoreReached events after run with p2 score event",
        );

        // Prime ourselves for a victory on next score, then simulate p1 win
        *world.get_resource_mut::<Score>().unwrap() = Score { p1: 9, p2: 9 };
        world
            .query::<(&ScoreText, &mut Text2d)>()
            .iter_mut(&mut world)
            .for_each(
                |(_, txt)| txt.into_inner().0 = "9".into(), // Prime ScoreTexts
            );
        world.send_event(PlayerScored(Player1));
        world.run_system(score_sys).unwrap();
        validate_scores(
            &mut world,
            10,
            9,
            "10",
            "9",
            true,
            false,
            "after run with p1 winning",
        );
        assert_eq!(
            world
                .get_resource::<Events<MaxScoreReached>>()
                .unwrap()
                .len(),
            1,
            "Expected 1 MaxScoreReached event after run with p1 winning",
        );
        world
            .get_resource_mut::<Events<MaxScoreReached>>()
            .unwrap()
            .clear(); // Clear for next test

        // Prime ourselves for a victory on next score, then simulate p2 win
        *world.get_resource_mut::<Score>().unwrap() = Score { p1: 9, p2: 9 };
        world
            .query_filtered::<&mut Text2d, With<ScoreText>>()
            .iter_mut(&mut world)
            .for_each(
                |txt| txt.into_inner().0 = "9".into(), // Prime ScoreTexts
            );
        world
            .query_filtered::<&mut Visibility, With<WinText>>()
            .iter_mut(&mut world)
            .for_each(
                |vis| *vis.into_inner() = Visibility::Hidden, // Prime WinTexts
            );
        world.send_event(PlayerScored(Player2));
        world.run_system(score_sys).unwrap();
        validate_scores(
            &mut world,
            9,
            10,
            "9",
            "10",
            false,
            true,
            "after run with p2 winning",
        );
        assert_eq!(
            world
                .get_resource::<Events<MaxScoreReached>>()
                .unwrap()
                .len(),
            1,
            "Expected 1 MaxScoreReached event after run with p2 winning",
        );
    }

    #[test]
    fn test_clear_scores_system() {
        // Create world with necessary resources
        let mut world = World::default();
        world.init_resource::<Events<ClearScores>>();
        world.init_resource::<Score>();

        // Systems we'll need for this test
        let cam_create_sys = world.register_system(
            // Create a camera (for setup sys)
            |mut commands: Commands| {
                commands.spawn(Camera2d);
            },
        );
        let setup_sys = world.register_system(setup); // Setup text entities in the world
        let clear_sys = world.register_system(clear_scores);

        // Prime the world by running our setup systems
        world.run_system(cam_create_sys).unwrap();
        world.run_system(setup_sys).unwrap();

        // Start by setting everything to a "non-cleared" state
        *world.get_resource_mut::<Score>().unwrap() = Score { p1: 10, p2: 10 };
        world
            .query_filtered::<&mut Text2d, With<ScoreText>>()
            .iter_mut(&mut world)
            .for_each(
                |txt| txt.into_inner().0 = "10".into(), // Prime ScoreTexts
            );
        world
            .query_filtered::<&mut Visibility, With<WinText>>()
            .iter_mut(&mut world)
            .for_each(
                |vis| *vis.into_inner() = Visibility::Visible, // Prime WinTexts
            );

        // Now run the clear system without any event input. Nothing should happen
        world.run_system(clear_sys).unwrap();
        validate_scores(
            &mut world,
            10,
            10,
            "10",
            "10",
            true,
            true,
            "after no clear events",
        );

        // And now send the event and confirm everything is wiped out
        world.send_event(ClearScores);
        world.run_system(clear_sys).unwrap();
        validate_scores(
            &mut world,
            0,
            0,
            "0",
            "0",
            false,
            false,
            "after sending clear event",
        );
    }

    // --- Helper Functions ---

    fn validate_scores(
        world: &mut World,
        p1: u8,
        p2: u8,
        p1_text: &str,
        p2_text: &str,
        p1_win: bool,
        p2_win: bool,
        log: &str,
    ) {
        assert_eq!(
            *world.get_resource::<Score>().unwrap(),
            Score { p1, p2 },
            "Expected score to be {}-{} {}",
            p1,
            p2,
            log,
        );

        // Get the ScoreText entities created by the setup system
        let mut query = world.query::<(&ScoreText, &Text2d)>();
        for (&ScoreText(id), Text2d(txt)) in query.iter(world) {
            let exp_val = if id == Player1 { p1_text } else { p2_text };
            assert_eq!(txt, exp_val, "Expected {id:?} score text '{exp_val}' {log}");
        }

        // Get the WinText entities created by the setup system
        let mut query = world.query::<(&WinText, &Visibility)>();
        for (&WinText(id), vis) in query.iter(world) {
            let exp_val = if id == Player1 { p1_win } else { p2_win };
            let exp_val = if exp_val {
                Visibility::Visible
            } else {
                Visibility::Hidden
            };
            assert_eq!(
                vis, exp_val,
                "Expected {id:?} visibility '{exp_val:?}' {log}"
            );
        }
    }
}
