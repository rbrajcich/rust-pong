//!
//! This module conatins code to manage the ball within the game, including
//! its creation, movement, and physics. It also exposes APIs to notify other
//! modules when the ball has left the screen, and reset the ball between/before rounds.
//!

// -------------------------------------------------------------------------------------------------
// Included Symbols

use std::f32::consts::PI;

use bevy::prelude::*;
use bevy::sprite::Anchor;
use rand::Rng;

use crate::common::*;
use crate::paddle::{self, AllPaddleHitboxes, PaddleHitbox, PaddleMarker};

// -------------------------------------------------------------------------------------------------
// Constants

const BALL_COLOR: Color = Color::srgb_u8(0, 255, 0);
const BALL_SIZE_AS_SCREEN_HEIGHT_PCT: f32 = 0.02;
const BALL_SPEED_AS_SCREEN_WIDTH_PCT: f32 = 0.9;
const BALL_SIZE: f32 = BALL_SIZE_AS_SCREEN_HEIGHT_PCT * ARENA_HEIGHT;
const BALL_SPEED: f32 = BALL_SPEED_AS_SCREEN_WIDTH_PCT * ARENA_WIDTH;
const BALL_OFF_SCREEN_X_MAG: f32 = (ARENA_WIDTH / 2f32) - (BALL_SIZE / 2f32);

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
                    move_and_collide.before(detect_ball_off_screen),
                    detect_ball_off_screen.in_set(Systems::BallOffScreenSndr),
                    handle_reset_ball.in_set(Systems::ResetBallRcvr),
                    handle_start_ball.in_set(Systems::StartBallRcvr),
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
#[derive(Event)]
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
        },
        Sprite {
            color: BALL_COLOR,
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
    ball_q: Single<(&mut Ball, &mut Transform), Without<PaddleMarker>>,
    paddles: Query<AllPaddleHitboxes>,
) {
    let (mut ball, mut ball_tf) = ball_q.into_inner();

    if !ball.paused {
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
        ball.paused = true;
        ball_tf.translation = Vec3::ZERO;
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
    // )
    let paddle = if ball.movement_dir.x > 0f32 {
        // Focus on collisions with p2 paddle if moving right
        let hitbox = PaddleHitbox::from_query(paddles, Player2);
        (
            hitbox.plane_origin() - Vec2::new(ball_rad, 0f32),
            Plane2d::new(Vec2::NEG_X),
            hitbox.bot_y() - ball_rad,
            hitbox.top_y() + ball_rad,
        )
    } else {
        // Otherwise, focus on p1 paddle
        let hitbox = PaddleHitbox::from_query(paddles, Player1);
        (
            hitbox.plane_origin() + Vec2::new(ball_rad, 0f32),
            Plane2d::new(Vec2::X),
            hitbox.bot_y() - ball_rad,
            hitbox.top_y() + ball_rad,
        )
    };

    let ball_ray = Ray2d::new(ball_tf.translation.xy(), ball.movement_dir);

    // (Distance to impact point, Normal, Cached impact point once computed)
    struct Collision(f32, Plane2d, Option<Vec2>);

    let mut paddle_collision: Option<Collision> = None;
    if let Some(dist) = ball_ray.intersect_plane(paddle.0, paddle.1) {
        if dist <= move_dist {
            let impact_point = ball_ray.get_point(dist);
            if (impact_point.y >= paddle.2) && (impact_point.y <= paddle.3) {
                paddle_collision = Some(Collision(dist, paddle.1, Some(impact_point)));
            }
        }
    }

    let mut wall_collision: Option<Collision> = None;
    if let Some(dist) = ball_ray.intersect_plane(wall.0, wall.1) {
        if dist <= move_dist {
            wall_collision = Some(Collision(dist, wall.1, None));
        }
    }

    let mut apply_collision = |collision: Collision| {
        let impact_point = collision.2.unwrap_or(ball_ray.get_point(collision.0));
        ball_tf.translation = impact_point.extend(0f32);
        ball.movement_dir =
            Dir2::new_unchecked(ball.movement_dir.reflect(collision.1.normal.as_vec2()));
        Some(move_dist - collision.0)
    };

    match (paddle_collision, wall_collision) {
        (Some(pad_imp), Some(wall_imp)) if pad_imp.0 < wall_imp.0 => apply_collision(pad_imp),
        (Some(pad_imp), Some(wall_imp)) if wall_imp.0 < pad_imp.0 => apply_collision(wall_imp),
        (Some(_pad_imp), Some(wall_imp)) => apply_collision(wall_imp), // TODO equals case???
        (None, Some(imp)) | (Some(imp), None) => apply_collision(imp),
        (None, None) => None,
    }
}
