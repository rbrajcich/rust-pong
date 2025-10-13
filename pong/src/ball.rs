//!
//! This module conatins code to manage the ball within the game, including
//! its creation, movement, and physics. It also exposes APIs to notify other
//! modules when the ball has left the screen, and reset the ball between/before rounds.
//!

// -------------------------------------------------------------------------------------------------
// Included Symbols

use std::f32::consts::PI;
use std::time::Duration;

use bevy::prelude::*;
use bevy::sprite::Anchor;
use rand::Rng;

use crate::common::*;
use crate::paddle::{self, AllPaddleHitboxes, Paddle, PaddleHitbox};

// -------------------------------------------------------------------------------------------------
// Constants

const BALL_SIZE_AS_SCREEN_HEIGHT_PCT: f32 = 0.02;
const BALL_SPEED_AS_SCREEN_WIDTH_PCT: f32 = 0.9;
const BALL_SIZE: f32 = BALL_SIZE_AS_SCREEN_HEIGHT_PCT * ARENA_HEIGHT;
const BALL_SPEED: f32 = BALL_SPEED_AS_SCREEN_WIDTH_PCT * ARENA_WIDTH;
const BALL_OFF_SCREEN_X_MAG: f32 = (ARENA_WIDTH / 2f32) - (BALL_SIZE / 2f32);

const BALL_CURVE_CFG_NONE: CurveLevelCfg = CurveLevelCfg {
    color: BallColor::Solid(Color::srgb_u8(0, 255, 0)),
    rotate_rad_per_sec: 0.0,
    curve_rad_per_sec: 0.0,
};
const BALL_CURVE_CFG_L1: CurveLevelCfg = CurveLevelCfg {
    color: BallColor::Solid(Color::srgb_u8(0, 255, 0)),
    rotate_rad_per_sec: 2.0 * PI,
    curve_rad_per_sec: 0.1 * PI,
};
const BALL_CURVE_CFG_L2: CurveLevelCfg = CurveLevelCfg {
    color: BallColor::Solid(Color::srgb_u8(255, 255, 0)),
    rotate_rad_per_sec: 3.0 * PI,
    curve_rad_per_sec: 0.3 * PI,
};
const BALL_CURVE_CFG_L3: CurveLevelCfg = CurveLevelCfg {
    color: BallColor::Blinking {
        blink_time: Duration::from_millis(230),
        colors: &[Color::srgb_u8(0, 255, 0), Color::srgb_u8(255, 255, 0)],
    },
    rotate_rad_per_sec: 5.0 * PI,
    curve_rad_per_sec: 0.6 * PI,
};
const BALL_CURVE_LEVELS: [CurveLevelCfg; 4] = [
    BALL_CURVE_CFG_NONE,
    BALL_CURVE_CFG_L1,
    BALL_CURVE_CFG_L2,
    BALL_CURVE_CFG_L3,
];

// -------------------------------------------------------------------------------------------------
// Public API

///
/// This plugin adds the pong ball to the screen, and implements all associated
/// functionality. It can be interacted with via various events defined in this module's API.
/// The exposed system sets should be used to constrain ordering as needed to ensure
/// same-frame responses between event triggers and reactionary systems.
///
pub struct BallPlugin;

impl Plugin for BallPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<BallOffScreen>()
            .add_event::<ResetBall>()
            .add_event::<StartBall>()
            .add_systems(Startup, setup_ball.in_set(Systems::BallCreation))
            .add_systems(
                Update,
                (
                    move_and_collide
                        .before(detect_ball_off_screen)
                        .before(apply_curve_visuals),
                    detect_ball_off_screen.in_set(Systems::BallOffScreenSndr),
                    handle_reset_ball
                        .in_set(Systems::ResetBallRcvr)
                        .before(apply_curve_visuals),
                    handle_start_ball.in_set(Systems::StartBallRcvr),
                    apply_curve_visuals,
                ),
            )
            .configure_sets(
                Update,
                paddle::Systems::HandleInput.before(move_and_collide),
            );
    }
}

///
/// Identifies the Ball entity in the game world. The component is exposed to allow disjoint
/// queries or basic access using Without<Ball> or With<Ball>. It should (and can't) be used
/// to construct a Ball outside this module.
///
#[derive(Component)]
pub struct Ball {
    // The current forward movement vector for the ball.
    movement_dir: Dir2,

    // Current paused state for the ball. It will not move when paused.
    paused: bool,

    // The current curve state of this ball.
    curve: CurveState,
}

///
/// System sets to allow modules consuming this plugin to create ordering constraints
/// based on functionality exposed in the API of the Plugin.
///
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum Systems {
    /// Startup systems which create the ball. After this, one Ball entity will exist.
    BallCreation,

    ///
    /// Update systems which send BallOffScreen events. To react to these events in the
    /// same frame, the receiver should be ordered after this system set.
    ///
    BallOffScreenSndr,

    ///
    /// Update systems which react to ResetBall events. To react to these events in the
    /// same frame, the sender should be ordered before this system set.
    ///
    ResetBallRcvr,

    ///
    /// Update systems which react to StartBall events. To react to these events in the
    /// same frame, the sender should be ordered before this system set.
    ///
    StartBallRcvr,
}

///
/// This event will be triggered by code in the BallPlugin to notify other modules
/// that the ball has reached the edge of the screen on the left or right side, without
/// bouncing off a paddle.
///
/// If a system needs to respond to this event in the same frame, it should be ordered
/// before the BallOffScreenSndr SystemSet.
///
#[derive(Event, Clone, Copy, PartialEq, Eq, Debug)]
pub enum BallOffScreen {
    Left,
    Right,
}

///
/// This event should be triggered by another module to signal that the ball should be
/// reset to its initial state. I.e. paused, and located in the middle of the screen.
///
/// If the reset needs to occur in the same frame as this event gets triggered, the
/// system generating the event should be ordered before ResetBallRcvr.
///
#[derive(Event)]
pub struct ResetBall;

///
/// This event should be triggered by another module to signal that the ball should
/// unpause and start moving in a random direction towards the left or right paddle.
///
/// If the start needs to occur in the same frame as this event gets triggered, the
/// system generating the event should be ordered before StartBallRcvr.
///
#[derive(Event)]
pub struct StartBall;

// -------------------------------------------------------------------------------------------------
// Private Types

// Represents a possible color (or blinking color sequence) for the ball.
enum BallColor<'a> {
    Solid(Color),
    Blinking {
        blink_time: Duration,
        colors: &'a [Color],
    },
}

// Represents a particular curve state/configuration to apply to the ball.
struct CurveLevelCfg<'a> {
    color: BallColor<'a>,
    rotate_rad_per_sec: f32, // Should always be positive
    curve_rad_per_sec: f32,  // Should always be positive
}

// Represents the direction the ball is currently curving in, if any.
#[derive(PartialEq, Eq, Default, Clone, Copy)]
enum CurveDir {
    #[default]
    None,
    Clockwise,
    CounterClockwise,
}

// Represents the overall state of curving applied to a ball.
#[derive(Default)]
struct CurveState {
    dir: CurveDir,
    cfg_idx: usize,
    color_timer: Timer,
    color_idx: usize,
}

impl CurveState {
    //
    // Given the current curve state, update it according to some event/collision that
    // has applied the new curve direction. This should either stop the curve, amplify it,
    // or change its direction and reset it to the first curve level.
    //
    fn apply_curve(&mut self, dir: CurveDir) {
        let prev_state = (self.dir, self.cfg_idx);
        if dir == CurveDir::None {
            self.dir = CurveDir::None;
            self.cfg_idx = 0;
        } else if dir == self.dir {
            // Same curve as already applied. Strengthen it if possible.
            if self.cfg_idx < (BALL_CURVE_LEVELS.len() - 1) {
                self.cfg_idx += 1;
            }
        } else {
            // Applying a new curve direction. Start at level 1.
            self.dir = dir;
            self.cfg_idx = 1;
        }

        // If we actually changed our curve level or direction, update ball accordingly
        if prev_state != (self.dir, self.cfg_idx) {
            let new_state = BALL_CURVE_LEVELS.get(self.cfg_idx).unwrap();
            match new_state.color {
                BallColor::Solid(_) => self.color_timer.pause(),
                BallColor::Blinking { blink_time, .. } => {
                    self.color_timer = Timer::new(blink_time, TimerMode::Repeating);
                    self.color_idx = 0;
                }
            }
        }
    }

    //
    // Get the current color that should be applied to the ball during the current frame.
    // Takes time_delta as input to update internal animation state as needed for this frame.
    //
    fn get_color(&mut self, time_delta: Duration) -> Color {
        let cur_state = BALL_CURVE_LEVELS.get(self.cfg_idx).unwrap();
        match cur_state.color {
            BallColor::Solid(color) => color,
            BallColor::Blinking { colors, .. } => {
                self.color_timer.tick(time_delta);
                self.color_idx += self.color_timer.times_finished_this_tick() as usize;
                self.color_idx %= colors.len();
                *colors.get(self.color_idx).unwrap()
            }
        }
    }

    //
    // Given the time_delta for the current frame, return how many radians the ball
    // should be rotated by according to its current curve state.
    //
    fn get_rotation_delta(&self, time_delta: Duration) -> f32 {
        let cur_state = BALL_CURVE_LEVELS.get(self.cfg_idx).unwrap();
        cur_state.rotate_rad_per_sec * time_delta.as_secs_f32()
    }

    //
    // Given the time_delta for the current frame, return how many radians the ball's
    // trajectory should be rotated by according to its current curve state.
    //
    fn get_trajectory_delta(&self, time_delta: Duration) -> f32 {
        let cur_state = BALL_CURVE_LEVELS.get(self.cfg_idx).unwrap();
        cur_state.curve_rad_per_sec * time_delta.as_secs_f32()
    }
}

// -------------------------------------------------------------------------------------------------
// Private Systems

//
// Adds the Ball entity to the app with the appropriate on-screen size and color.
// It initially starts paused in the center with no movement vector
//
fn setup_ball(mut commands: Commands) {
    commands.spawn((
        Ball {
            movement_dir: Dir2::X,
            paused: true,
            curve: CurveState::default(),
        },
        Sprite {
            custom_size: Some(Vec2::ONE),
            anchor: Anchor::Center,
            ..default()
        },
        Transform::from_scale(Vec3::new(BALL_SIZE, BALL_SIZE, 0f32)),
    ));
}

//
// This system updates the ball's movement each frame, and applies any collisions with
// the edge of the arena or with a paddle, as needed. It runs after any user input
// to ensure we check collision with the most recent paddle positions.
//
fn move_and_collide(
    time: Res<Time>,
    ball_q: Single<(&mut Ball, &mut Transform), Without<Paddle>>,
    paddles: Query<AllPaddleHitboxes>,
) {
    let (mut ball, mut ball_tf) = ball_q.into_inner();

    if !ball.paused {
        // Update trajectory based on curve
        let trajectory_delta = match ball.curve.dir {
            CurveDir::Clockwise => Mat2::from_angle(-ball.curve.get_trajectory_delta(time.delta())),
            CurveDir::CounterClockwise => {
                Mat2::from_angle(ball.curve.get_trajectory_delta(time.delta()))
            }
            CurveDir::None => Mat2::IDENTITY,
        };
        ball.movement_dir = Dir2::new(trajectory_delta * ball.movement_dir.as_vec2()).unwrap();

        // Move the ball along its trajectory and collide as needed
        let mut move_dist = time.delta_secs() * BALL_SPEED;
        loop {
            let collision_dist = collide_once(move_dist, &mut ball, &mut ball_tf, paddles);
            match collision_dist {
                Some(dist) => move_dist -= dist,
                None => break,
            };
        }
        let movement_vec = ball.movement_dir * move_dist;
        ball_tf.translation += movement_vec.extend(0f32);
    }
}

//
// This system updates the ball's Sprite's visual appearance each frame based on the current
// curve defined in CurveState, including color and rotation.
//
fn apply_curve_visuals(time: Res<Time>, ball_q: Single<(&mut Ball, &mut Sprite, &mut Transform)>) {
    let (mut ball, mut sprite, mut ball_tf) = ball_q.into_inner();

    // Update the color of the ball based on current curve state
    let color = ball.curve.get_color(time.delta());
    sprite.color = color;

    // Update visual rotation of the ball's sprite based on current curve state
    let rotation_delta = match ball.curve.dir {
        CurveDir::Clockwise => -ball.curve.get_rotation_delta(time.delta()),
        CurveDir::CounterClockwise => ball.curve.get_rotation_delta(time.delta()),
        CurveDir::None => 0f32,
    };
    ball_tf.rotation *= Quat::from_rotation_z(rotation_delta);
}

//
// Notifies other modules that the ball has reached the edge of the screen, by
// dispatching BallOffScreen events.
//
fn detect_ball_off_screen(
    ball_q: Single<(&mut Ball, &mut Transform)>,
    mut event_writer: EventWriter<BallOffScreen>,
) {
    let (ball, ball_tf) = ball_q.into_inner();

    if ball.paused {
        return;
    }

    if ball_tf.translation.x.abs() > BALL_OFF_SCREEN_X_MAG {
        // Ball has collided with left/right wall! Raise event
        event_writer.write(if ball_tf.translation.x.is_sign_positive() {
            BallOffScreen::Right
        } else {
            BallOffScreen::Left
        });
    }
}

//
// Handles ResetBall events sent by other modules, to pause the Ball and reset it to
// its initial state in the center of the screen.
//
fn handle_reset_ball(
    mut events: EventReader<ResetBall>,
    ball_q: Single<(&mut Ball, &mut Transform)>,
) {
    if !events.is_empty() {
        events.clear();

        let (mut ball, mut ball_tf) = ball_q.into_inner();
        ball.curve.apply_curve(CurveDir::None);
        ball.paused = true;
        ball_tf.translation.x = 0f32;
        ball_tf.translation.y = 0f32;
        ball_tf.rotation = Quat::IDENTITY;
    }
}

//
// Handles StartBall events sent by other modules, to unpause the Ball and
// start it moving in a random direction towards the left or right wall.
//
fn handle_start_ball(mut events: EventReader<StartBall>, ball_q: Single<&mut Ball>) {
    if !events.is_empty() {
        events.clear();

        // Generate a random starting angle (w/ 50% change of each direction)
        let mut rng = rand::rng();
        let random_angle = rng.random_range(-(PI / 7f32)..(PI / 7f32));
        let mut rotation_quat = Quat::from_rotation_z(random_angle);
        if rng.random_bool(1.0 / 2.0) {
            // flip rotation 180 degrees
            rotation_quat *= Quat::from_rotation_z(PI);
        }

        let mut ball = ball_q.into_inner();
        ball.movement_dir = Dir2::new_unchecked((rotation_quat * Vec3::X).xy());
        ball.paused = false;
    }
}

// -------------------------------------------------------------------------------------------------
// Private Functions

//
// Attempts to collide the ball once with the nearest surface (wall or paddle). This
// function will move the ball to the collision point and update its movement vector.
// If a collision occurred, Some(f32) will be returned with the distance that
// the ball has moved to reach this collision point. None is returned for no
// collision. Ideally, this function should be called repeatedly until None is returned.
//
fn collide_once(
    move_dist: f32,
    ball: &mut Ball,
    ball_tf: &mut Transform,
    paddles: Query<AllPaddleHitboxes>,
) -> Option<f32> {
    // How far from center of ball should it "collide" with objects
    let ball_rad = ball_tf.scale.x / 2f32;

    // (Plane origin offset for ball size, Plane)
    let wall = if ball.movement_dir.y > 0f32 {
        // Focus on collisions with top wall if moving up
        (
            Vec2::new(0f32, (ARENA_HEIGHT / 2f32) - ball_rad),
            Plane2d::new(Vec2::NEG_Y),
        )
    } else {
        // Otherwise, bottom wall
        (
            Vec2::new(0f32, (-ARENA_HEIGHT / 2f32) + ball_rad),
            Plane2d::new(Vec2::Y),
        )
    };

    // (
    //     Plane origin offset for ball size,
    //     Plane,
    //     Paddle bot offset for ball size,
    //     Paddle top offset for ball size,
    //     Applied spin on ball,
    // )
    let paddle = if ball.movement_dir.x > 0f32 {
        // Focus on collisions with p2 paddle if moving right
        let hitbox = PaddleHitbox::from_query(paddles, Player2);
        (
            hitbox.plane_origin() - Vec2::new(ball_rad, 0f32),
            Plane2d::new(Vec2::NEG_X),
            hitbox.bot_y() - ball_rad,
            hitbox.top_y() + ball_rad,
            match hitbox.movement_dir() {
                paddle::MoveDirection::Up => CurveDir::CounterClockwise,
                paddle::MoveDirection::Down => CurveDir::Clockwise,
                paddle::MoveDirection::None => CurveDir::None,
            },
        )
    } else {
        // Otherwise, focus on p1 paddle
        let hitbox = PaddleHitbox::from_query(paddles, Player1);
        (
            hitbox.plane_origin() + Vec2::new(ball_rad, 0f32),
            Plane2d::new(Vec2::X),
            hitbox.bot_y() - ball_rad,
            hitbox.top_y() + ball_rad,
            match hitbox.movement_dir() {
                paddle::MoveDirection::Up => CurveDir::Clockwise,
                paddle::MoveDirection::Down => CurveDir::CounterClockwise,
                paddle::MoveDirection::None => CurveDir::None,
            },
        )
    };

    let ball_ray = Ray2d::new(ball_tf.translation.xy(), ball.movement_dir);

    // (Distance to impact point, Normal, CurveDir if applies, Cached impact point once computed)
    struct Collision(f32, Plane2d, Option<CurveDir>, Option<Vec2>);

    let mut paddle_collision: Option<Collision> = None;
    if let Some(dist) = ball_ray.intersect_plane(paddle.0, paddle.1) {
        if dist <= move_dist {
            let impact_point = ball_ray.get_point(dist);
            if (impact_point.y >= paddle.2) && (impact_point.y <= paddle.3) {
                paddle_collision = Some(Collision(
                    dist,
                    paddle.1,
                    Some(paddle.4),
                    Some(impact_point),
                ));
            }
        }
    }

    let mut wall_collision: Option<Collision> = None;
    if let Some(dist) = ball_ray.intersect_plane(wall.0, wall.1) {
        if dist <= move_dist {
            wall_collision = Some(Collision(dist, wall.1, None, None));
        }
    }

    let mut apply_collision = |collision: Collision| {
        let impact_point = collision.3.unwrap_or(ball_ray.get_point(collision.0));
        ball_tf.translation = impact_point.extend(0f32);
        ball.movement_dir =
            Dir2::new_unchecked(ball.movement_dir.reflect(collision.1.normal.as_vec2()));
        if let Some(curve_dir) = collision.2 {
            ball.curve.apply_curve(curve_dir);
        }
        Some(collision.0)
    };

    match (paddle_collision, wall_collision) {
        (Some(pad_imp), Some(wall_imp)) if pad_imp.0 < wall_imp.0 => apply_collision(pad_imp),
        (Some(pad_imp), Some(wall_imp)) if wall_imp.0 < pad_imp.0 => apply_collision(wall_imp),
        (Some(pad_imp), Some(wall_imp)) => {
            // Hitting wall and paddle at same dist (corner)
            apply_collision(wall_imp);
            apply_collision(pad_imp)
        }
        (None, Some(imp)) | (Some(imp), None) => apply_collision(imp),
        (None, None) => None,
    }
}

// -------------------------------------------------------------------------------------------------
// Unit Tests

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::schedule::AnonymousSet;
    use bevy_test_helpers::prelude::*;
    use std::time::Duration;

    #[test]
    fn test_plugin_build() {
        let mut app = App::new();
        app.add_plugins(BallPlugin);

        let world = app.world();
        assert!(
            world.is_resource_added::<Events<BallOffScreen>>(),
            "Expected BallOffScreen events to be added by BallPlugin",
        );
        assert!(
            world.is_resource_added::<Events<StartBall>>(),
            "Expected StartBall events to be added by BallPlugin",
        );
        assert!(
            world.is_resource_added::<Events<ResetBall>>(),
            "Expected ResetBall events to be added by BallPlugin",
        );
    }

    #[test]
    fn test_plugin_added_sys_setup() {
        validate_sys_in_plugin(BallPlugin, Startup, setup_ball, Some(Systems::BallCreation));
    }

    #[test]
    fn test_plugin_added_sys_move() {
        validate_sys_in_plugin(
            BallPlugin,
            Update,
            move_and_collide,
            Option::<AnonymousSet>::None,
        );
    }

    #[test]
    fn test_plugin_added_sys_detect_off_screen() {
        validate_sys_in_plugin(
            BallPlugin,
            Update,
            detect_ball_off_screen,
            Some(Systems::BallOffScreenSndr),
        );
    }

    #[test]
    fn test_plugin_added_sys_handle_reset() {
        validate_sys_in_plugin(
            BallPlugin,
            Update,
            handle_reset_ball,
            Some(Systems::ResetBallRcvr),
        );
    }

    #[test]
    fn test_plugin_added_sys_handle_start() {
        validate_sys_in_plugin(
            BallPlugin,
            Update,
            handle_start_ball,
            Some(Systems::StartBallRcvr),
        );
    }

    #[test]
    fn test_setup_system() {
        let mut world = World::default();

        // Run the system
        let setup_sys = world.register_system(setup_ball);
        world.run_system(setup_sys).unwrap();

        // Validate ball created as expected
        let mut query = world.query::<(&Ball, &Sprite, &Transform)>();
        let (ball, sprite, ball_tf) = query.single(&world).unwrap_or_else(|err| {
            panic!(
                "Expected successful query for single ball. Got error {:?}",
                err,
            );
        });
        assert!(ball.paused, "Expected ball to start in paused state");
        let size = sprite
            .custom_size
            .expect("Expected custom size of 1x1 for ball sprite");
        assert_eq!(
            size,
            Vec2::ONE,
            "Expected ball sprite to have size 1x1, got {}",
            size,
        );
        assert_eq!(
            sprite.anchor,
            Anchor::Center,
            "Expected ball sprite anchored in center, got {:?}",
            sprite.anchor
        );
        assert_eq!(
            ball_tf.scale,
            Vec3::new(BALL_SIZE, BALL_SIZE, 0f32),
            "Expected Ball to be BALL_SIZE x BALL_SIZE x 0, but got {}",
            ball_tf.scale,
        );
    }

    #[test]
    fn test_move_while_paused() {
        test_move_and_collide_helper(&TestMoveCollideCfg {
            paused: true,
            time_deltas: &[Duration::from_millis(100)],
            init_pos: Vec2::ZERO,
            init_dir: Dir2::X,
            p1_paddle_ends: (0f32, 0f32),
            p2_paddle_ends: (0f32, 0f32),
            exp_pos: Vec2::ZERO,
            exp_dir: Dir2::X,
        });
    }

    #[test]
    fn test_move_collide_left() {
        // Solid collision with paddle
        let exp_collision_x =
            (-ARENA_WIDTH / 2.0) + paddle::tests::get_paddle_width() + (BALL_SIZE / 2.0);
        let exp_collision_y = 0.0;

        test_move_and_collide_helper(&TestMoveCollideCfg {
            paused: false,

            // Time so that distance after collision is half of pre-collision
            time_deltas: &[Duration::from_secs_f32((5.0 / BALL_SPEED) * 1.5)],

            // Use known 3/4/5 triangle for pre-collision vector for simplicity
            init_pos: Vec2::new(exp_collision_x + 4.0, exp_collision_y - 3.0),
            init_dir: Dir2::from_xy(-4.0, 3.0).unwrap(),

            p1_paddle_ends: (1.0, -1.0),
            p2_paddle_ends: (0.0, 0.0),

            // Post reflection vector should be half of pre-collision 3/4/5 triangle
            exp_pos: Vec2::new(exp_collision_x + 2.0, exp_collision_y + 1.5),
            exp_dir: Dir2::from_xy(4.0, 3.0).unwrap(),
        });
    }

    #[test]
    fn test_move_collide_left_with_time_stop() {
        // Perform a single collision, but with 2 executions of the move/collide system.
        // The time stops right at the moment of collision, to ensure exactly 1 collision occurs.
        let exp_collision_x =
            (-ARENA_WIDTH / 2.0) + paddle::tests::get_paddle_width() + (BALL_SIZE / 2.0);
        let exp_collision_y = 0.0;

        test_move_and_collide_helper(&TestMoveCollideCfg {
            paused: false,

            // Time so that distance after collision is half of pre-collision
            time_deltas: &[
                Duration::from_secs_f32(5.0 / BALL_SPEED),
                Duration::from_secs_f32((5.0 / BALL_SPEED) * 0.5),
            ],

            // Use known 3/4/5 triangle for pre-collision vector for simplicity
            init_pos: Vec2::new(exp_collision_x + 4.0, exp_collision_y - 3.0),
            init_dir: Dir2::from_xy(-4.0, 3.0).unwrap(),

            p1_paddle_ends: (1.0, -1.0),
            p2_paddle_ends: (0.0, 0.0),

            // Post reflection vector should be half of pre-collision 3/4/5 triangle
            exp_pos: Vec2::new(exp_collision_x + 2.0, exp_collision_y + 1.5),
            exp_dir: Dir2::from_xy(4.0, 3.0).unwrap(),
        });
    }

    #[test]
    fn test_move_collide_barely_left() {
        // Barely collide with corner of paddle
        let exp_collision_x =
            (-ARENA_WIDTH / 2.0) + paddle::tests::get_paddle_width() + (BALL_SIZE / 2.0);
        let exp_collision_y = 0.0 + (BALL_SIZE / 4.0); // 1/4 of ball overlapping paddle edge

        test_move_and_collide_helper(&TestMoveCollideCfg {
            paused: false,

            // Time so that distance after collision is half of pre-collision
            time_deltas: &[Duration::from_secs_f32((5.0 / BALL_SPEED) * 1.5)],

            // Use known 3/4/5 triangle for pre-collision vector for simplicity
            init_pos: Vec2::new(exp_collision_x + 4.0, exp_collision_y + 3.0),
            init_dir: Dir2::from_xy(-4.0, -3.0).unwrap(),

            p1_paddle_ends: (0.0, -1.0),
            p2_paddle_ends: (0.0, 0.0),

            // Post reflection vector should be half of pre-collision 3/4/5 triangle
            exp_pos: Vec2::new(exp_collision_x + 2.0, exp_collision_y - 1.5),
            exp_dir: Dir2::from_xy(4.0, -3.0).unwrap(),
        });
    }

    #[test]
    fn test_move_miss_barely_left() {
        // Barely miss the edge of the paddle
        let exp_intersect_x =
            (-ARENA_WIDTH / 2.0) + paddle::tests::get_paddle_width() + (BALL_SIZE / 2.0);
        let exp_intersect_y = 0.0 + (BALL_SIZE / 2.0) + 0.001;

        test_move_and_collide_helper(&TestMoveCollideCfg {
            paused: false,

            // Time long enough to pass would-be collision point
            time_deltas: &[Duration::from_secs_f32((5.0 / BALL_SPEED) * 2.0)],

            // Use known 3/4/5 triangle for pre-intersection vector for simplicity
            init_pos: Vec2::new(exp_intersect_x + 4.0, exp_intersect_y - 3.0),
            init_dir: Dir2::from_xy(-4.0, 3.0).unwrap(),

            p1_paddle_ends: (0.0, -1.0),
            p2_paddle_ends: (0.0, 0.0),

            // Should continue traveling past intersection point without collision
            exp_pos: Vec2::new(exp_intersect_x - 4.0, exp_intersect_y + 3.0),
            exp_dir: Dir2::from_xy(-4.0, 3.0).unwrap(),
        });
    }

    #[test]
    fn test_move_collide_right() {
        // Solid collision with paddle
        let exp_collision_x =
            (ARENA_WIDTH / 2.0) - paddle::tests::get_paddle_width() - (BALL_SIZE / 2.0);
        let exp_collision_y = 0.0;

        test_move_and_collide_helper(&TestMoveCollideCfg {
            paused: false,

            // Time so that distance after collision is half of pre-collision
            time_deltas: &[Duration::from_secs_f32((5.0 / BALL_SPEED) * 1.5)],

            // Use known 3/4/5 triangle for pre-collision vector for simplicity
            init_pos: Vec2::new(exp_collision_x - 4.0, exp_collision_y - 3.0),
            init_dir: Dir2::from_xy(4.0, 3.0).unwrap(),

            p1_paddle_ends: (0.0, 0.0),
            p2_paddle_ends: (1.0, -1.0),

            // Post reflection vector should be half of pre-collision 3/4/5 triangle
            exp_pos: Vec2::new(exp_collision_x - 2.0, exp_collision_y + 1.5),
            exp_dir: Dir2::from_xy(-4.0, 3.0).unwrap(),
        });
    }

    #[test]
    fn test_move_collide_right_with_time_stop() {
        // Perform a single collision, but with 2 executions of the move/collide system.
        // The time stops right at the moment of collision, to ensure exactly 1 collision occurs.
        let exp_collision_x =
            (ARENA_WIDTH / 2.0) - paddle::tests::get_paddle_width() - (BALL_SIZE / 2.0);
        let exp_collision_y = 0.0;

        test_move_and_collide_helper(&TestMoveCollideCfg {
            paused: false,

            // Time so that distance after collision is half of pre-collision
            time_deltas: &[
                Duration::from_secs_f32(5.0 / BALL_SPEED),
                Duration::from_secs_f32((5.0 / BALL_SPEED) * 0.5),
            ],

            // Use known 3/4/5 triangle for pre-collision vector for simplicity
            init_pos: Vec2::new(exp_collision_x - 4.0, exp_collision_y - 3.0),
            init_dir: Dir2::from_xy(4.0, 3.0).unwrap(),

            p1_paddle_ends: (0.0, 0.0),
            p2_paddle_ends: (1.0, -1.0),

            // Post reflection vector should be half of pre-collision 3/4/5 triangle
            exp_pos: Vec2::new(exp_collision_x - 2.0, exp_collision_y + 1.5),
            exp_dir: Dir2::from_xy(-4.0, 3.0).unwrap(),
        });
    }

    #[test]
    fn test_move_collide_barely_right() {
        // Barely collide with corner of paddle
        let exp_collision_x =
            (ARENA_WIDTH / 2.0) - paddle::tests::get_paddle_width() - (BALL_SIZE / 2.0);
        let exp_collision_y = 0.0 + (BALL_SIZE / 4.0); // 1/4 of ball overlapping paddle edge

        test_move_and_collide_helper(&TestMoveCollideCfg {
            paused: false,

            // Time so that distance after collision is half of pre-collision
            time_deltas: &[Duration::from_secs_f32((5.0 / BALL_SPEED) * 1.5)],

            // Use known 3/4/5 triangle for pre-collision vector for simplicity
            init_pos: Vec2::new(exp_collision_x - 4.0, exp_collision_y + 3.0),
            init_dir: Dir2::from_xy(4.0, -3.0).unwrap(),

            p1_paddle_ends: (0.0, 0.0),
            p2_paddle_ends: (0.0, -1.0),

            // Post reflection vector should be half of pre-collision 3/4/5 triangle
            exp_pos: Vec2::new(exp_collision_x - 2.0, exp_collision_y - 1.5),
            exp_dir: Dir2::from_xy(-4.0, -3.0).unwrap(),
        });
    }

    #[test]
    fn test_move_miss_barely_right() {
        // Barely miss the edge of the paddle
        let exp_intersect_x =
            (ARENA_WIDTH / 2.0) - paddle::tests::get_paddle_width() - (BALL_SIZE / 2.0);
        let exp_intersect_y = 0.0 + (BALL_SIZE / 2.0) + 0.001;

        test_move_and_collide_helper(&TestMoveCollideCfg {
            paused: false,

            // Time long enough to pass would-be collision point
            time_deltas: &[Duration::from_secs_f32((5.0 / BALL_SPEED) * 2.0)],

            // Use known 3/4/5 triangle for pre-intersection vector for simplicity
            init_pos: Vec2::new(exp_intersect_x - 4.0, exp_intersect_y - 3.0),
            init_dir: Dir2::from_xy(4.0, 3.0).unwrap(),

            p1_paddle_ends: (0.0, 0.0),
            p2_paddle_ends: (0.0, -1.0),

            // Should continue traveling past intersection point without collision
            exp_pos: Vec2::new(exp_intersect_x + 4.0, exp_intersect_y + 3.0),
            exp_dir: Dir2::from_xy(4.0, 3.0).unwrap(),
        });
    }

    #[test]
    fn test_move_collide_top() {
        // Solid collision with top of arena
        let exp_collision_x = 0.0;
        let exp_collision_y = (ARENA_HEIGHT / 2.0) - (BALL_SIZE / 2.0);

        test_move_and_collide_helper(&TestMoveCollideCfg {
            paused: false,

            // Time so that distance after collision is half of pre-collision
            time_deltas: &[Duration::from_secs_f32((5.0 / BALL_SPEED) * 1.5)],

            // Use known 3/4/5 triangle for pre-collision vector for simplicity
            init_pos: Vec2::new(exp_collision_x - 4.0, exp_collision_y - 3.0),
            init_dir: Dir2::from_xy(4.0, 3.0).unwrap(),

            p1_paddle_ends: (0.0, 0.0),
            p2_paddle_ends: (0.0, 0.0),

            // Post reflection vector should be half of pre-collision 3/4/5 triangle
            exp_pos: Vec2::new(exp_collision_x + 2.0, exp_collision_y - 1.5),
            exp_dir: Dir2::from_xy(4.0, -3.0).unwrap(),
        });
    }

    #[test]
    fn test_move_collide_bottom() {
        // Solid collision with bottom of arena
        let exp_collision_x = 0.0;
        let exp_collision_y = (-ARENA_HEIGHT / 2.0) + (BALL_SIZE / 2.0);

        test_move_and_collide_helper(&TestMoveCollideCfg {
            paused: false,

            // Time so that distance after collision is half of pre-collision
            time_deltas: &[Duration::from_secs_f32((5.0 / BALL_SPEED) * 1.5)],

            // Use known 3/4/5 triangle for pre-collision vector for simplicity
            init_pos: Vec2::new(exp_collision_x - 4.0, exp_collision_y + 3.0),
            init_dir: Dir2::from_xy(4.0, -3.0).unwrap(),

            p1_paddle_ends: (0.0, 0.0),
            p2_paddle_ends: (0.0, 0.0),

            // Post reflection vector should be half of pre-collision 3/4/5 triangle
            exp_pos: Vec2::new(exp_collision_x + 2.0, exp_collision_y + 1.5),
            exp_dir: Dir2::from_xy(4.0, 3.0).unwrap(),
        });
    }

    #[test]
    fn test_move_collide_paddle_wall() {
        // Collide with p2 paddle then top wall
        // Init point to collision1 is 3/4/5 triangle.
        // Collision1 to collision2 is 3/4/5 triangle too.
        let exp_collision2_y = (ARENA_HEIGHT / 2.0) - (BALL_SIZE / 2.0);
        let exp_collision1_x =
            (ARENA_WIDTH / 2.0) - paddle::tests::get_paddle_width() - (BALL_SIZE / 2.0);
        let exp_collision1_y = exp_collision2_y - 3.0;
        let exp_collision2_x = exp_collision1_x - 4.0;

        test_move_and_collide_helper(&TestMoveCollideCfg {
            paused: false,

            // Time for 2 collisions of length 5, plus 1/2 that dist after 2nd collision
            time_deltas: &[Duration::from_secs_f32((5.0 / BALL_SPEED) * 2.5)],

            init_pos: Vec2::new(exp_collision1_x - 4.0, exp_collision1_y - 3.0),
            init_dir: Dir2::from_xy(4.0, 3.0).unwrap(),

            p1_paddle_ends: (0.0, 0.0),
            p2_paddle_ends: (ARENA_HEIGHT / 2.0, -ARENA_HEIGHT / 2.0),

            // After second collision vector should be half the distance based on time
            exp_pos: Vec2::new(exp_collision2_x - 2.0, exp_collision2_y - 1.5),
            exp_dir: Dir2::from_xy(-4.0, -3.0).unwrap(),
        });
    }

    #[test]
    fn test_move_collide_wall_paddle() {
        // Collide with top wall, then p2 paddle
        // Init point to collision1 is 3/4/5 triangle.
        // Collision1 to collision2 is 3/4/5 triangle too.
        let exp_collision1_y = (ARENA_HEIGHT / 2.0) - (BALL_SIZE / 2.0);
        let exp_collision2_x =
            (ARENA_WIDTH / 2.0) - paddle::tests::get_paddle_width() - (BALL_SIZE / 2.0);
        let exp_collision2_y = exp_collision1_y - 3.0;
        let exp_collision1_x = exp_collision2_x - 4.0;

        test_move_and_collide_helper(&TestMoveCollideCfg {
            paused: false,

            // Time for 2 collisions of length 5, plus 1/2 that dist after 2nd collision
            time_deltas: &[Duration::from_secs_f32((5.0 / BALL_SPEED) * 2.5)],

            init_pos: Vec2::new(exp_collision1_x - 4.0, exp_collision1_y - 3.0),
            init_dir: Dir2::from_xy(4.0, 3.0).unwrap(),

            p1_paddle_ends: (0.0, 0.0),
            p2_paddle_ends: (ARENA_HEIGHT / 2.0, -ARENA_HEIGHT / 2.0),

            // After second collision vector should be half the distance based on time
            exp_pos: Vec2::new(exp_collision2_x - 2.0, exp_collision2_y - 1.5),
            exp_dir: Dir2::from_xy(-4.0, -3.0).unwrap(),
        });
    }

    #[test]
    fn test_move_collide_corner() {
        // Collide with "corner" of arena, hitting wall and paddle at exact same point
        let exp_collision_x =
            (ARENA_WIDTH / 2.0) - paddle::tests::get_paddle_width() - (BALL_SIZE / 2.0);
        let exp_collision_y = (ARENA_HEIGHT / 2.0) - (BALL_SIZE / 2.0);

        test_move_and_collide_helper(&TestMoveCollideCfg {
            paused: false,

            // Time for 1 collision 5 units away, plus 1/2 that dist afterwards
            time_deltas: &[Duration::from_secs_f32((5.0 / BALL_SPEED) * 1.5)],

            // Use known 3/4/5 triangle for pre-collision vector for simplicity
            init_pos: Vec2::new(exp_collision_x - 4.0, exp_collision_y - 3.0),
            init_dir: Dir2::from_xy(4.0, 3.0).unwrap(),

            p1_paddle_ends: (0.0, 0.0),
            p2_paddle_ends: (ARENA_HEIGHT / 2.0, -ARENA_HEIGHT / 2.0),

            // Post reflection vector should be half of pre-collision 3/4/5 triangle
            exp_pos: Vec2::new(exp_collision_x - 2.0, exp_collision_y - 1.5),
            exp_dir: Dir2::from_xy(-4.0, -3.0).unwrap(),
        });
    }

    #[test]
    fn test_ball_off_screen_sys_paused() {
        test_ball_off_screen_helper(true, BALL_OFF_SCREEN_X_MAG * 2f32, None);
    }

    #[test]
    fn test_ball_off_screen_sys_left() {
        test_ball_off_screen_helper(
            false,
            -(BALL_OFF_SCREEN_X_MAG + 1f32),
            Some(BallOffScreen::Left),
        );
    }

    #[test]
    fn test_ball_off_screen_sys_right() {
        test_ball_off_screen_helper(
            false,
            BALL_OFF_SCREEN_X_MAG + 1f32,
            Some(BallOffScreen::Right),
        );
    }

    #[test]
    fn test_ball_off_screen_sys_neither() {
        test_ball_off_screen_helper(false, BALL_OFF_SCREEN_X_MAG - 1f32, None);
    }

    #[test]
    fn test_reset_ball_sys() {
        let mut world = World::default();

        // Spawn Ball in the world
        world.spawn((
            Ball {
                movement_dir: Dir2::X,
                paused: false,
                curve: CurveState::default(),
            },
            Transform {
                translation: Vec3::new(45f32, -102f32, 8f32),
                ..default()
            },
        ));

        // Create event and resource containing it, for system to receive
        let mut events = Events::<ResetBall>::default();
        events.send(ResetBall);
        world.insert_resource(events);

        // Run the system
        let reset_sys = world.register_system(handle_reset_ball);
        world.run_system(reset_sys).unwrap();

        // Validate Ball was reset
        let mut query = world.query::<(&Ball, &Transform)>();
        let (ball, ball_tf) = query.single(&world).unwrap_or_else(|err| {
            panic!("Attempt to query single Ball failed with err {err}");
        });
        assert!(ball.paused, "Expected ball to be paused after reset");
        assert_eq!(
            ball_tf.translation,
            Vec3::new(0f32, 0f32, 8f32),
            "Expected Ball translation of {} but got {}",
            Vec3::new(0f32, 0f32, 8f32),
            ball_tf.translation,
        );
    }

    #[test]
    fn test_start_ball_sys() {
        let mut world = World::default();

        // Spawn Ball in the world
        world.spawn((
            Ball {
                movement_dir: Dir2::X,
                paused: true,
                curve: CurveState::default(),
            },
            Transform::default(),
        ));

        // Create event and resource containing it, for system to receive
        let mut events = Events::<StartBall>::default();
        events.send(StartBall);
        world.insert_resource(events);

        // Run the system
        let start_sys = world.register_system(handle_start_ball);
        world.run_system(start_sys).unwrap();

        // Validate Ball was started (note we ignore direction part, since it's random)
        let mut query = world.query::<&Ball>();
        let ball = query.single(&world).unwrap_or_else(|err| {
            panic!("Attempt to query single Ball failed with err {err}");
        });
        assert!(
            !ball.paused,
            "Expected ball to be unpaused after start event"
        );
    }

    // --- Helper Types ---

    struct TestMoveCollideCfg<'a> {
        paused: bool,
        time_deltas: &'a [Duration],
        init_pos: Vec2,
        init_dir: Dir2,
        p1_paddle_ends: (f32, f32), // Y coordinates of top and bottom
        p2_paddle_ends: (f32, f32), // Y coordinates of top and bottom
        exp_pos: Vec2,
        exp_dir: Dir2,
    }

    // --- Helper Functions ---

    fn test_move_and_collide_helper(cfg: &TestMoveCollideCfg) {
        let mut world = World::default();

        // Spawn Paddles and Ball based on Config, and add Time resource and system
        paddle::tests::spawn_test_paddle(
            &mut world,
            cfg.p1_paddle_ends.0,
            cfg.p1_paddle_ends.1,
            Player1,
        );
        paddle::tests::spawn_test_paddle(
            &mut world,
            cfg.p2_paddle_ends.0,
            cfg.p2_paddle_ends.1,
            Player2,
        );
        world.spawn((
            Ball {
                movement_dir: cfg.init_dir,
                paused: cfg.paused,
                curve: CurveState::default(),
            },
            Transform {
                translation: cfg.init_pos.extend(0f32),
                scale: Vec2::splat(BALL_SIZE).extend(0f32),
                ..default()
            },
        ));
        world.init_resource::<Time>();
        let move_sys = world.register_system(move_and_collide);

        for delta in cfg.time_deltas {
            // Set up Time resource to simulate configured time delta
            let mut time = world.get_resource_mut::<Time>().unwrap();
            time.advance_by(*delta);

            // Run the move/collision system
            world.run_system(move_sys).unwrap();
        }

        // Validate the new position and direction of the ball
        let mut query = world.query::<(&Ball, &Transform)>();
        let (ball, ball_tf) = query.single(&world).unwrap_or_else(|err| {
            panic!("Expected single query of Ball to succeed, but got err {err}");
        });
        assert!(
            (ball.movement_dir.x - cfg.exp_dir.x).abs() < 0.00001,
            "Expected movement dir x coordinate of {} but got {}",
            cfg.exp_dir.x,
            ball.movement_dir.x,
        );
        assert!(
            (ball.movement_dir.y - cfg.exp_dir.y).abs() < 0.00001,
            "Expected movement dir y coordinate of {} but got {}",
            cfg.exp_dir.y,
            ball.movement_dir.y,
        );
        assert!(
            (ball_tf.translation.x - cfg.exp_pos.x).abs() < 0.00001,
            "Expected ball pos x coordinate of {} but got {}",
            cfg.exp_pos.x,
            ball_tf.translation.x,
        );
        assert!(
            (ball_tf.translation.y - cfg.exp_pos.y).abs() < 0.00001,
            "Expected ball pos y coordinate of {} but got {}",
            cfg.exp_pos.y,
            ball_tf.translation.y,
        );
    }

    fn test_ball_off_screen_helper(
        ball_paused: bool,
        ball_x: f32,
        expected_event: Option<BallOffScreen>,
    ) {
        let mut world = World::default();

        // Spawn Ball in the world given the input parameters
        world.spawn((
            Ball {
                movement_dir: Dir2::X,
                paused: ball_paused,
                curve: CurveState::default(),
            },
            Transform {
                translation: Vec3::new(ball_x, 0f32, 0f32),
                ..default()
            },
        ));

        // Add the BallOffScreen event resource for the system to write to
        world.init_resource::<Events<BallOffScreen>>();

        // Run the system
        let detect_sys = world.register_system(detect_ball_off_screen);
        world.run_system(detect_sys).unwrap();

        // Validate if the event was generated or not
        let events = world.get_resource::<Events<BallOffScreen>>().unwrap();
        let mut evt_cursor = events.get_cursor();
        let mut evt_iter = evt_cursor.read(&events);
        if expected_event.is_none() {
            assert!(
                evt_iter.next().is_none(),
                "Expected no BallOffScreen event, but got one",
            );
        } else {
            let received_evt = *evt_iter
                .next()
                .expect("Expected a BallOffScreen event, but got none");
            assert_eq!(
                received_evt,
                expected_event.unwrap(),
                "Expected event {:?} but got event {:?}",
                expected_event.unwrap(),
                received_evt,
            );
        }
    }
}
