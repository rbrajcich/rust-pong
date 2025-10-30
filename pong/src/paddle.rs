//!
//! Contains code to setup and manage the paddles on either side of the pong screen,
//! and allow other code to query for paddle positional data for use in collision logic.
//!

// -------------------------------------------------------------------------------------------------
// Included Symbols

use bevy::ecs::query::QueryData;
use bevy::prelude::*;
use bevy::sprite::Anchor;

use crate::common::*;

// -------------------------------------------------------------------------------------------------
// Constants

const PADDLE_HEIGHT_AS_SCREEN_PCT: f32 = 0.15;
const PADDLE_ASPECT_RATIO: f32 = 0.15;
const PADDLE_MOVE_SPEED: f32 = ARENA_HEIGHT * 1.5;
const PADDLE_HEIGHT: f32 = PADDLE_HEIGHT_AS_SCREEN_PCT * ARENA_HEIGHT;
const PADDLE_WIDTH: f32 = PADDLE_HEIGHT * PADDLE_ASPECT_RATIO;
const PADDLE_CLAMP_Y: f32 = (ARENA_HEIGHT / 2f32) - (PADDLE_HEIGHT / 2f32);

// -------------------------------------------------------------------------------------------------
// Public API

///
/// The PaddlePlugin adds 2 paddles to the screen, one on each side.
/// It also handles user input to move the paddles up and down using W/S and ^/v keys.
/// There is also a read-only API exposed to query positional data about the paddles
/// for use in collision computation.
///
pub struct PaddlePlugin;

impl Plugin for PaddlePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_paddles.in_set(Systems::PaddleCreation))
            .add_systems(
                Update,
                handle_input_move_paddles.in_set(Systems::HandleInput),
            );
    }
}

/// These SystemSets are used to control any system ordering dependencies on this plugin
#[derive(SystemSet, Debug, Clone, Hash, PartialEq, Eq)]
pub enum Systems {
    /// Creates the paddle entities. Must be in Startup.
    PaddleCreation,

    ///
    /// Implements all logic to retrieve user input events and update
    /// the paddle hitbox and latest movement data accordingly. Must be in Update.
    ///
    HandleInput,
}

///
/// Read-only (to public API users) component which is present on paddle entities.
/// Intended for use by other code modules to help avoid query component conflicts,
/// by using Without<Paddle> in Query filters as needed.
///
#[derive(Component)]
pub struct Paddle {
    player: PlayerId,
    move_dir: MoveDirection,
}

impl Paddle {
    // Private constructor to easily create a Paddle with given player Id and other defaults.
    fn new(player: PlayerId) -> Self {
        Paddle {
            player,
            move_dir: MoveDirection::None,
        }
    }
}

///
/// A custom QueryData which allows read-only access to the hitbox API.
/// The entrypoint for the API is a system with parameter Query<AllPaddleHitboxes>.
/// From there, the API allows an individual player hitbox to be selected,
/// and relevant data for the hitbox can be retrieved via the API.
///
#[derive(QueryData)]
pub struct AllPaddleHitboxes(&'static Paddle, &'static Transform);

///
/// A type alias to allow more succinct access to the individual hitbox "items"
/// within AllPaddleHitboxes. The type itself represents a single paddle hitbox
/// within the world. It allows retrieval of several relevant hitbox-related values
/// of the paddle to be used in collision detection.
///
pub type PaddleHitbox<'w, 's> = AllPaddleHitboxesItem<'w, 's>;

impl<'w, 's> PaddleHitbox<'w, 's> {
    ///
    /// Given the query for all paddle hitboxes, retrieve the one specific to a
    /// particular PlayerId.
    ///
    pub fn from_query(query: Query<'w, 's, AllPaddleHitboxes>, player: PlayerId) -> Self {
        for item in query {
            if item.0.player == player {
                return item as PaddleHitbox;
            }
        }
        panic!("PlayerId {player:?} was not found in AllPaddleHitboxes query.");
    }

    ///
    /// Get an origin point for the collision plane of this paddle. The plane
    /// is on the vertical face of the paddle nearest the center line of the arena.
    ///
    pub fn plane_origin(&self) -> Vec2 {
        let x_offset = match self.0.player {
            Player1 => self.1.scale.x,
            Player2 => -self.1.scale.x,
        };

        self.1.translation.xy() + Vec2::new(x_offset, 0f32)
    }

    /// Get the topmost Y coordinate of the collision surface of the paddle.
    pub fn top_y(&self) -> f32 {
        self.1.translation.y + (self.1.scale.y / 2f32)
    }

    /// Get the bottommost Y coordinate of the collision surface of the paddle.
    pub fn bot_y(&self) -> f32 {
        self.1.translation.y - (self.1.scale.y / 2f32)
    }

    /// Get the direction this paddle moved in the most recent update.
    pub fn movement_dir(&self) -> MoveDirection {
        self.0.move_dir
    }
}

///
/// Represents any of the possible directions that a paddle may have moved in the most
/// recent frame of the game.
///
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MoveDirection {
    /// Paddle was stationary during last update.
    None,
    /// Paddle moved towards positive Y direction last update.
    Up,
    /// Paddle moved towards negative Y direction last update.
    Down,
}

// -------------------------------------------------------------------------------------------------
// Private Systems

//
// Creates two paddles - one for each player. One paddle is against the left edge of
// the screen, one is against the right edge. They are vertically centered to start.
//
fn setup_paddles(mut commands: Commands) {
    let paddle_size = Vec3::new(PADDLE_WIDTH, PADDLE_HEIGHT, 0f32);

    commands.spawn((
        Paddle::new(Player1),
        Sprite {
            color: Color::WHITE,
            custom_size: Some(Vec2::ONE),
            ..default()
        },
        Anchor::CENTER_LEFT,
        Transform {
            translation: Vec3 {
                x: -ARENA_WIDTH / 2f32,
                y: 0f32,
                z: Z_FOREGROUND,
            },
            scale: paddle_size,
            ..default()
        },
    ));

    commands.spawn((
        Paddle::new(Player2),
        Sprite {
            color: Color::WHITE,
            custom_size: Some(Vec2::ONE),
            ..default()
        },
        Anchor::CENTER_RIGHT,
        Transform {
            translation: Vec3 {
                x: ARENA_WIDTH / 2f32,
                y: 0f32,
                z: Z_FOREGROUND,
            },
            scale: paddle_size,
            ..default()
        },
    ));
}

// Checks relevant user inputs and updates positions of paddles accordingly.
fn handle_input_move_paddles(
    paddles: Query<(&mut Transform, &mut Paddle)>,
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
) {
    let distance = time.delta_secs() * PADDLE_MOVE_SPEED;
    let ((p1_trans, p1_move_dir), (p2_trans, p2_move_dir)) = paddles
        .into_iter()
        .map(|(t, pad)| {
            (
                pad.player,
                (
                    &mut t.into_inner().translation,
                    &mut pad.into_inner().move_dir,
                ),
            )
        })
        .as_per_player();

    match (keys.pressed(KeyCode::KeyW), keys.pressed(KeyCode::KeyS)) {
        (true, false) => {
            if p1_trans.y < PADDLE_CLAMP_Y {
                p1_trans.y = (p1_trans.y + distance).min(PADDLE_CLAMP_Y);
                *p1_move_dir = MoveDirection::Up;
            } else {
                *p1_move_dir = MoveDirection::None;
            }
        }
        (false, true) => {
            if p1_trans.y > -PADDLE_CLAMP_Y {
                p1_trans.y = (p1_trans.y - distance).max(-PADDLE_CLAMP_Y);
                *p1_move_dir = MoveDirection::Down;
            } else {
                *p1_move_dir = MoveDirection::None;
            }
        }
        _ => *p1_move_dir = MoveDirection::None, // No p1 movement if neither or both are pressed
    }

    match (
        keys.pressed(KeyCode::ArrowUp),
        keys.pressed(KeyCode::ArrowDown),
    ) {
        (true, false) => {
            if p2_trans.y < PADDLE_CLAMP_Y {
                p2_trans.y = (p2_trans.y + distance).min(PADDLE_CLAMP_Y);
                *p2_move_dir = MoveDirection::Up;
            } else {
                *p2_move_dir = MoveDirection::None;
            }
        }
        (false, true) => {
            if p2_trans.y > -PADDLE_CLAMP_Y {
                p2_trans.y = (p2_trans.y - distance).max(-PADDLE_CLAMP_Y);
                *p2_move_dir = MoveDirection::Down;
            } else {
                *p2_move_dir = MoveDirection::None;
            }
        }
        _ => *p2_move_dir = MoveDirection::None, // No p2 movement if neither or both are pressed
    }
}

// -------------------------------------------------------------------------------------------------
// Unit Tests

#[cfg(test)]
pub mod tests {
    use super::*;
    use bevy_test_helpers::prelude::*;
    use std::time::Duration;

    #[test]
    fn test_plugin_sys_added_setup() {
        validate_sys_in_plugin(
            PaddlePlugin,
            Startup,
            setup_paddles,
            Some(Systems::PaddleCreation),
        );
    }

    #[test]
    fn test_plugin_sys_added_handle_input() {
        validate_sys_in_plugin(
            PaddlePlugin,
            Update,
            handle_input_move_paddles,
            Some(Systems::HandleInput),
        );
    }

    #[test]
    fn test_setup_paddles_system() {
        let mut world = World::default();

        // Run the system and let it create entities we expect
        let setup_sys = world.register_system(setup_paddles);
        world.run_system(setup_sys).unwrap();

        // Show Without<Paddle> works to guarantee disjoint queries
        let mut query = world.query_filtered::<&Transform, Without<Paddle>>();
        assert_eq!(
            query.iter(&world).count(),
            0,
            "Expected no items in query when using filter Without<Paddle>"
        );

        // Validate paddles are created with sensible values.
        let mut query_state = world.query::<(&Paddle, &Sprite, &Anchor, &Transform)>();
        let query = query_state.query(&world);
        assert_eq!(
            query.iter().len(),
            2,
            "Expected 2 paddles to be added by setup system",
        );
        let mut seen_pid: Option<PlayerId> = None;
        for (&Paddle { player: pid, .. }, sprite, anchor, tf) in query {
            // Confirm the paddles have different PlayerId values.
            match seen_pid {
                None => seen_pid = Some(pid),
                Some(seen_pid) => assert_ne!(
                    seen_pid, pid,
                    "Expected each paddle to be different PlayerId",
                ),
            }

            // Some common properties that both paddles should have
            assert_eq!(
                sprite.custom_size,
                Some(Vec2::ONE),
                "Expected sprite size of 1, so transform scale is real effective size",
            );
            assert_eq!(
                tf.scale,
                Vec3::new(PADDLE_WIDTH, PADDLE_HEIGHT, 0f32),
                "Expected paddle size {}x{}x0 but got {}",
                PADDLE_WIDTH,
                PADDLE_HEIGHT,
                tf.scale,
            );
            assert_eq!(
                tf.translation.y, 0f32,
                "Expected paddle at y of zero (vertically centered). Got {}",
                tf.translation.y,
            );
            assert_eq!(
                tf.translation.z, Z_FOREGROUND,
                "Expected paddle to be at foreground Z coord of {}, but got {}",
                Z_FOREGROUND, tf.translation.z,
            );

            // Last couple validations, which are done per-paddle.
            match pid {
                Player1 => {
                    assert_eq!(
                        *anchor,
                        Anchor::CENTER_LEFT,
                        "Expected P1 paddle anchored at Center Left, got {:?}",
                        *anchor,
                    );
                    assert_eq!(
                        tf.translation.x,
                        -ARENA_WIDTH / 2f32,
                        "Expected P1 paddle x at left edge of screen, {}, but got {}",
                        -ARENA_WIDTH / 2f32,
                        tf.translation.x,
                    );
                }
                Player2 => {
                    assert_eq!(
                        *anchor,
                        Anchor::CENTER_RIGHT,
                        "Expected P2 paddle anchored at Center Right, got {:?}",
                        *anchor,
                    );
                    assert_eq!(
                        tf.translation.x,
                        ARENA_WIDTH / 2f32,
                        "Expected P2 paddle x at right edge of screen, {}, but got {}",
                        ARENA_WIDTH / 2f32,
                        tf.translation.x,
                    );
                }
            }
        }
    }

    #[test]
    fn test_handle_input_no_keys_down() {
        run_handle_input_scenario(
            0f32,
            0f32,
            Duration::from_millis(5),
            [].as_slice(),
            0f32,
            0f32,
            MoveDirection::None,
            MoveDirection::None,
        );
    }

    #[test]
    fn test_handle_input_w_down() {
        run_handle_input_scenario(
            0f32,
            0f32,
            Duration::from_millis(5),
            [KeyCode::KeyW, KeyCode::ArrowDown].as_slice(),
            0.005 * PADDLE_MOVE_SPEED,
            -0.005 * PADDLE_MOVE_SPEED,
            MoveDirection::Up,
            MoveDirection::Down,
        );
    }

    #[test]
    fn test_handle_input_s_up() {
        run_handle_input_scenario(
            0f32,
            0f32,
            Duration::from_millis(5),
            [KeyCode::KeyS, KeyCode::ArrowUp].as_slice(),
            -0.005 * PADDLE_MOVE_SPEED,
            0.005 * PADDLE_MOVE_SPEED,
            MoveDirection::Down,
            MoveDirection::Up,
        );
    }

    #[test]
    fn test_handle_input_both_dirs_pressed() {
        run_handle_input_scenario(
            0f32,
            0f32,
            Duration::from_millis(5),
            [
                KeyCode::KeyS,
                KeyCode::KeyW,
                KeyCode::ArrowDown,
                KeyCode::ArrowUp,
            ]
            .as_slice(),
            0f32,
            0f32,
            MoveDirection::None,
            MoveDirection::None,
        );
    }

    #[test]
    fn test_handle_input_existing_positive_cap() {
        run_handle_input_scenario(
            (ARENA_HEIGHT / 2f32) - (PADDLE_HEIGHT / 2f32),
            (ARENA_HEIGHT / 2f32) - (PADDLE_HEIGHT / 2f32),
            Duration::from_millis(5),
            [KeyCode::KeyW, KeyCode::ArrowUp].as_slice(),
            (ARENA_HEIGHT / 2f32) - (PADDLE_HEIGHT / 2f32),
            (ARENA_HEIGHT / 2f32) - (PADDLE_HEIGHT / 2f32),
            MoveDirection::None,
            MoveDirection::None,
        );
    }

    #[test]
    fn test_handle_input_existing_negative_cap() {
        run_handle_input_scenario(
            (-ARENA_HEIGHT / 2f32) + (PADDLE_HEIGHT / 2f32),
            (-ARENA_HEIGHT / 2f32) + (PADDLE_HEIGHT / 2f32),
            Duration::from_millis(5),
            [KeyCode::KeyS, KeyCode::ArrowDown].as_slice(),
            (-ARENA_HEIGHT / 2f32) + (PADDLE_HEIGHT / 2f32),
            (-ARENA_HEIGHT / 2f32) + (PADDLE_HEIGHT / 2f32),
            MoveDirection::None,
            MoveDirection::None,
        );
    }

    #[test]
    fn test_handle_input_hitting_positive_cap() {
        run_handle_input_scenario(
            0f32,
            0f32,
            Duration::from_secs(5),
            [KeyCode::KeyW, KeyCode::ArrowUp].as_slice(),
            (ARENA_HEIGHT / 2f32) - (PADDLE_HEIGHT / 2f32),
            (ARENA_HEIGHT / 2f32) - (PADDLE_HEIGHT / 2f32),
            MoveDirection::Up,
            MoveDirection::Up,
        );
    }

    #[test]
    fn test_handle_input_hitting_negative_cap() {
        run_handle_input_scenario(
            0f32,
            0f32,
            Duration::from_secs(5),
            [KeyCode::KeyS, KeyCode::ArrowDown].as_slice(),
            (-ARENA_HEIGHT / 2f32) + (PADDLE_HEIGHT / 2f32),
            (-ARENA_HEIGHT / 2f32) + (PADDLE_HEIGHT / 2f32),
            MoveDirection::Down,
            MoveDirection::Down,
        );
    }

    #[test]
    fn test_hitbox_api() {
        let mut world = World::default();

        // Run a system to place a couple paddles in the world
        let setup_sys = world.register_system(|mut commands: Commands| {
            commands.spawn((
                Paddle {
                    player: Player2,
                    move_dir: MoveDirection::Up,
                },
                Transform {
                    translation: Vec3::new(10f32, 3f32, 0f32),
                    scale: Vec3::new(PADDLE_WIDTH, PADDLE_HEIGHT, 0f32),
                    ..default()
                },
            ));
            commands.spawn((
                Paddle {
                    player: Player1,
                    move_dir: MoveDirection::Down,
                },
                Transform {
                    translation: Vec3::new(-5f32, 8f32, 0f32),
                    scale: Vec3::new(PADDLE_WIDTH, PADDLE_HEIGHT, 0f32),
                    ..default()
                },
            ));
        });
        world.run_system(setup_sys).unwrap();

        // Get our Hitbox query handle
        let mut query_state = world.query::<AllPaddleHitboxes>();
        let hitbox_query = query_state.query(&world);

        // Validate player 1 hitbox parameters
        let p1_hitbox = PaddleHitbox::from_query(hitbox_query, Player1);
        let exp_top_y = 8f32 + (PADDLE_HEIGHT / 2f32);
        let exp_bot_y = 8f32 - (PADDLE_HEIGHT / 2f32);
        let exp_plane_x = -5f32 + PADDLE_WIDTH;
        assert_eq!(
            p1_hitbox.top_y(),
            exp_top_y,
            "Expected p1 hitbox top at y coord {}, but got {}",
            exp_top_y,
            p1_hitbox.top_y(),
        );
        assert_eq!(
            p1_hitbox.bot_y(),
            exp_bot_y,
            "Expected p1 hitbox bottom at y coord {}, but got {}",
            exp_bot_y,
            p1_hitbox.bot_y(),
        );
        assert_eq!(
            p1_hitbox.plane_origin().x,
            exp_plane_x,
            "Expected p1 plane origin x coord {}, but got {}",
            exp_plane_x,
            p1_hitbox.plane_origin().x,
        );
        assert!(
            (p1_hitbox.plane_origin().y <= exp_top_y) && (p1_hitbox.plane_origin().y >= exp_bot_y),
            "Expected p1 plane origin y coord between {} and {}, but got {}",
            exp_bot_y,
            exp_top_y,
            p1_hitbox.plane_origin().y,
        );
        assert_eq!(
            p1_hitbox.movement_dir(),
            MoveDirection::Down,
            "Expected p1 movement direction of Down",
        );

        // Validate player 2 hitbox parameters
        let p2_hitbox = PaddleHitbox::from_query(hitbox_query, Player2);
        let exp_top_y = 3f32 + (PADDLE_HEIGHT / 2f32);
        let exp_bot_y = 3f32 - (PADDLE_HEIGHT / 2f32);
        let exp_plane_x = 10f32 - PADDLE_WIDTH;
        assert_eq!(
            p2_hitbox.top_y(),
            exp_top_y,
            "Expected p2 hitbox top at y coord {}, but got {}",
            exp_top_y,
            p2_hitbox.top_y(),
        );
        assert_eq!(
            p2_hitbox.bot_y(),
            exp_bot_y,
            "Expected p2 hitbox bottom at y coord {}, but got {}",
            exp_bot_y,
            p2_hitbox.bot_y(),
        );
        assert_eq!(
            p2_hitbox.plane_origin().x,
            exp_plane_x,
            "Expected p2 plane origin x coord {}, but got {}",
            exp_plane_x,
            p2_hitbox.plane_origin().x,
        );
        assert!(
            (p2_hitbox.plane_origin().y <= exp_top_y) && (p2_hitbox.plane_origin().y >= exp_bot_y),
            "Expected p2 plane origin y coord between {} and {}, but got {}",
            exp_bot_y,
            exp_top_y,
            p2_hitbox.plane_origin().y,
        );
        assert_eq!(
            p2_hitbox.movement_dir(),
            MoveDirection::Up,
            "Expected p2 movement direction of Up",
        );
    }

    // ----- Helper Functions -----

    fn run_handle_input_scenario(
        init_p1_y: f32,
        init_p2_y: f32,
        time_delta: Duration,
        keys_pressed: &[KeyCode],
        exp_p1_y: f32,
        exp_p2_y: f32,
        exp_p1_dir: MoveDirection,
        exp_p2_dir: MoveDirection,
    ) {
        let mut world = World::default();

        // Set up some stand-in paddles for the test
        spawn_test_paddle(
            &mut world,
            init_p1_y + (PADDLE_HEIGHT / 2f32),
            init_p1_y - (PADDLE_HEIGHT / 2f32),
            Player1,
        );
        spawn_test_paddle(
            &mut world,
            init_p2_y + (PADDLE_HEIGHT / 2f32),
            init_p2_y - (PADDLE_HEIGHT / 2f32),
            Player2,
        );

        // Insert resource with specified time delta
        let mut time: Time<()> = Time::default();
        time.advance_by(time_delta);
        world.insert_resource(time);

        // Insert resource with specified keys "pressed"
        let mut button_input = ButtonInput::<KeyCode>::default();
        for key in keys_pressed {
            button_input.press(*key);
        }
        world.insert_resource(button_input);

        // Run system to move paddles
        let handle_input_sys = world.register_system(handle_input_move_paddles);
        world.run_system(handle_input_sys).unwrap();

        // Validate y positions are updated to expected values
        let mut query = world.query::<(&Paddle, &Transform)>();
        let (p1_tf, p2_tf) = query
            .iter(&world)
            .map(|(p, tf)| (p.player, tf))
            .as_per_player();
        assert_eq!(
            p1_tf.translation.y, exp_p1_y,
            "Expected p1 y to be {exp_p1_y} but it was {}",
            p1_tf.translation.y,
        );
        assert_eq!(
            p2_tf.translation.y, exp_p2_y,
            "Expected p2 y to be {exp_p2_y} but it was {}",
            p2_tf.translation.y,
        );

        // Validate movement directions are updated to expected values
        let mut query = world.query::<&Paddle>();
        let (p1_dir, p2_dir) = query
            .iter(&world)
            .map(|p| (p.player, p.move_dir))
            .as_per_player();
        assert_eq!(exp_p1_dir, p1_dir, "Expected p1 dir of {:?}", exp_p1_dir);
        assert_eq!(exp_p2_dir, p2_dir, "Expected p2 dir of {:?}", exp_p2_dir);
    }

    // --- External API For Other Test Suites ---
    pub fn spawn_test_paddle(world: &mut World, top_y: f32, bot_y: f32, player: PlayerId) {
        let x = match player {
            Player1 => -ARENA_WIDTH / 2f32,
            Player2 => ARENA_WIDTH / 2f32,
        };

        assert!(top_y >= bot_y, "Expected top_y to be greater than bot_y");

        let paddle_height = top_y - bot_y;
        let paddle_y = bot_y + (paddle_height / 2f32);

        world.spawn((
            Paddle::new(player),
            Transform {
                translation: Vec3 {
                    x: x,
                    y: paddle_y,
                    z: 0f32,
                },
                scale: Vec3::new(PADDLE_WIDTH, paddle_height, 0f32),
                ..default()
            },
        ));
    }

    pub fn get_paddle_width() -> f32 {
        return PADDLE_WIDTH;
    }
}
