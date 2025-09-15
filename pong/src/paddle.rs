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

pub const PADDLE_HEIGHT_AS_SCREEN_PCT: f32 = 0.15;
pub const PADDLE_ASPECT_RATIO: f32 = 0.15;
pub const PADDLE_MOVE_SPEED: f32 = ARENA_HEIGHT * 1.5;
pub const PADDLE_HEIGHT: f32 = PADDLE_HEIGHT_AS_SCREEN_PCT * ARENA_HEIGHT;
pub const PADDLE_WIDTH: f32 = PADDLE_HEIGHT * PADDLE_ASPECT_RATIO;
pub const PADDLE_CLAMP_Y: f32 = (ARENA_HEIGHT / 2f32) - (PADDLE_HEIGHT / 2f32);

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
        app.add_systems(Startup, setup_paddles.in_set(Systems::Startup))
            .add_systems(
                Update,
                handle_input_move_paddles.in_set(Systems::HandleInput),
            );
    }
}

/// These SystemSets are used to control any system ordering dependencies on this plugin
#[derive(SystemSet, Debug, Clone, Hash, PartialEq, Eq)]
pub enum Systems {
    /// Implements all logic to create the paddle entities. Must be in Startup.
    Startup,

    ///
    /// Implements all logic to retrieve user input events and update
    /// the paddle positions accordingly. Must be in Update.
    ///
    HandleInput,
}

///
/// Read-only marker component which is present on paddle entities.
/// Intended for use by other code modules to help avoid query component conflicts,
/// by using Without<PaddleMarker> in Query filters as needed.
///
#[derive(Component)]
pub struct PaddleMarker(PlayerId);

///
/// A custom QueryData which allows read-only access to the hitbox API.
/// The entrypoint for the API is a system with parameter Query<AllPaddleHitboxes>.
/// From there, the API allows an individual player hitbox to be selected,
/// and relevant data for the hitbox can be retrieved via the API.
///
#[derive(QueryData)]
pub struct AllPaddleHitboxes(&'static PaddleMarker, &'static Transform);

///
/// A type alias to allow more succinct access to the individual hitbox "items"
/// within AllPaddleHitboxes. The type itself represents a single paddle hitbox
/// within the world. It allows retrieval of several relevant hitbox-related values
/// of the paddle to be used in collision detection.
///
pub type PaddleHitbox<'w> = AllPaddleHitboxesItem<'w>;

impl<'w> PaddleHitbox<'w> {
    ///
    /// Given the query for all paddle hitboxes, retrieve the one specific to a
    /// particular PlayerId.
    ///
    pub fn from_query(query: Query<'w, '_, AllPaddleHitboxes>, player: PlayerId) -> Self {
        for item in query {
            if item.0.0 == player {
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
        let x_offset = match self.0.0 {
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
        PaddleMarker(Player1),
        Sprite {
            color: Color::WHITE,
            custom_size: Some(Vec2::ONE),
            anchor: Anchor::CenterLeft,
            ..default()
        },
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
        PaddleMarker(Player2),
        Sprite {
            color: Color::WHITE,
            custom_size: Some(Vec2::ONE),
            anchor: Anchor::CenterRight,
            ..default()
        },
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
    paddles: Query<(&mut Transform, &PaddleMarker)>,
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
) {
    let distance = time.delta_secs() * PADDLE_MOVE_SPEED;
    let (p1_trans, p2_trans) = paddles
        .into_iter()
        .map(|(t, pad)| (pad.0, &mut t.into_inner().translation))
        .as_per_player();

    match (keys.pressed(KeyCode::KeyW), keys.pressed(KeyCode::KeyS)) {
        (true, false) => {
            p1_trans.y = (p1_trans.y + distance).min(PADDLE_CLAMP_Y);
        }
        (false, true) => {
            p1_trans.y = (p1_trans.y - distance).max(-PADDLE_CLAMP_Y);
        }
        _ => (), // No p1 movement if neither or both are pressed
    }

    match (
        keys.pressed(KeyCode::ArrowUp),
        keys.pressed(KeyCode::ArrowDown),
    ) {
        (true, false) => {
            p2_trans.y = (p2_trans.y + distance).min(PADDLE_CLAMP_Y);
        }
        (false, true) => {
            p2_trans.y = (p2_trans.y - distance).max(-PADDLE_CLAMP_Y);
        }
        _ => (), // No p2 movement if neither or both are pressed
    }
}

// -------------------------------------------------------------------------------------------------
// Unit Tests

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::schedule::ScheduleBuildError;
    use std::time::Duration;

    #[test]
    fn test_sys_add_setup() {
        let mut app = App::new();
        app.add_plugins(PaddlePlugin);

        // This ordering will lead to an error (which we expect) if the system
        // exists and is in the system set as it should be.
        app.configure_sets(Startup, Systems::Startup.before(setup_paddles));
        let init_result = app
            .world_mut()
            .try_schedule_scope(Startup, |world, sched| sched.initialize(world))
            .expect("Expected Startup schedule to exist in app");
        let Err(ScheduleBuildError::SetsHaveOrderButIntersect(..)) = init_result else {
            panic!(concat!(
                "Expected Startup schedule build to fail, ",
                "since 'setup_paddles' should be in Startup system set. But it succeeded"
            ));
        };
    }

    #[test]
    fn test_sys_add_handle_input() {
        let mut app = App::new();
        app.add_plugins(PaddlePlugin);

        // This ordering will lead to an error (which we expect) if the system
        // exists and is in the system set as it should be.
        app.configure_sets(
            Update,
            Systems::HandleInput.before(handle_input_move_paddles),
        );
        let init_result = app
            .world_mut()
            .try_schedule_scope(Update, |world, sched| sched.initialize(world))
            .expect("Expected Update schedule to exist in app");
        let Err(ScheduleBuildError::SetsHaveOrderButIntersect(..)) = init_result else {
            panic!(concat!(
                "Expected Update schedule build to fail, ",
                "since 'handle_input_move_paddles' should be in Startup system set. ",
                "But it succeeded",
            ));
        };
    }

    #[test]
    fn test_setup_paddles_system() {
        let mut world = World::default();

        // Run the system and let it create entities we expect
        let setup_sys = world.register_system(setup_paddles);
        world.run_system(setup_sys).unwrap();

        // Show Without<PaddleMarker> works to guarantee disjoint queries
        let mut query = world.query_filtered::<&Transform, Without<PaddleMarker>>();
        assert_eq!(
            query.iter(&world).count(),
            0,
            "Expected no items in query when using filter Without<PaddleMarker>"
        );

        // Validate paddles are created with sensible values.
        let mut query_state = world.query::<(&PaddleMarker, &Sprite, &Transform)>();
        let query = query_state.query(&world);
        assert_eq!(
            query.iter().len(),
            2,
            "Expected 2 paddles to be added by setup system",
        );
        let mut seen_pid: Option<PlayerId> = None;
        for (&PaddleMarker(pid), sprite, tf) in query {
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
                        sprite.anchor,
                        Anchor::CenterLeft,
                        "Expected P1 paddle anchored at CenterLeft, got {:?}",
                        sprite.anchor,
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
                        sprite.anchor,
                        Anchor::CenterRight,
                        "Expected P2 paddle anchored at CenterRight, got {:?}",
                        sprite.anchor,
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
        run_handle_input_scenario(Duration::from_millis(5), [].as_slice(), 0f32, 0f32);
    }

    #[test]
    fn test_handle_input_w_down() {
        run_handle_input_scenario(
            Duration::from_millis(5),
            [KeyCode::KeyW, KeyCode::ArrowDown].as_slice(),
            0.005 * PADDLE_MOVE_SPEED,
            -0.005 * PADDLE_MOVE_SPEED,
        );
    }

    #[test]
    fn test_handle_input_s_up() {
        run_handle_input_scenario(
            Duration::from_millis(5),
            [KeyCode::KeyS, KeyCode::ArrowUp].as_slice(),
            -0.005 * PADDLE_MOVE_SPEED,
            0.005 * PADDLE_MOVE_SPEED,
        );
    }

    #[test]
    fn test_handle_input_both_dirs_pressed() {
        run_handle_input_scenario(
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
        );
    }

    #[test]
    fn test_handle_input_positive_cap() {
        run_handle_input_scenario(
            Duration::from_secs(5),
            [KeyCode::KeyW, KeyCode::ArrowUp].as_slice(),
            (ARENA_HEIGHT / 2f32) - (PADDLE_HEIGHT / 2f32),
            (ARENA_HEIGHT / 2f32) - (PADDLE_HEIGHT / 2f32),
        );
    }

    #[test]
    fn test_handle_input_negative_cap() {
        run_handle_input_scenario(
            Duration::from_secs(5),
            [KeyCode::KeyS, KeyCode::ArrowDown].as_slice(),
            (-ARENA_HEIGHT / 2f32) + (PADDLE_HEIGHT / 2f32),
            (-ARENA_HEIGHT / 2f32) + (PADDLE_HEIGHT / 2f32),
        );
    }

    #[test]
    fn test_hitbox_api() {
        let mut world = World::default();

        // Run a system to place a couple paddles in the world
        let setup_sys = world.register_system(|mut commands: Commands| {
            commands.spawn((
                PaddleMarker(Player2),
                Transform {
                    translation: Vec3::new(10f32, 3f32, 0f32),
                    scale: Vec3::new(PADDLE_WIDTH, PADDLE_HEIGHT, 0f32),
                    ..default()
                },
            ));
            commands.spawn((
                PaddleMarker(Player1),
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
    }

    // ----- Helper Functions -----

    fn run_handle_input_scenario(
        time_delta: Duration,
        keys_pressed: &[KeyCode],
        p1_y: f32,
        p2_y: f32,
    ) {
        let mut world = World::default();

        // Run system to set up paddles
        let setup_sys = world.register_system(setup_paddles);
        world.run_system(setup_sys).unwrap();

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
        let mut query = world.query::<(&PaddleMarker, &Transform)>();
        let (p1_tf, p2_tf) = query
            .iter(&world)
            .map(|(pm, tf)| (pm.0, tf))
            .as_per_player();
        assert_eq!(
            p1_tf.translation.y, p1_y,
            "Expected p1 y to be {p1_y} but it was {}",
            p1_tf.translation.y,
        );
        assert_eq!(
            p2_tf.translation.y, p2_y,
            "Expected p2 y to be {p2_y} but it was {}",
            p2_tf.translation.y,
        );
    }
}
