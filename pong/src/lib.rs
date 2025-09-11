mod arena;
mod common;
mod paddle;
mod score;

use bevy::sprite::Anchor;
use bevy::window::PresentMode;
use bevy::window::WindowResolution;
use bevy::prelude::*;
use std::f32::consts::PI;
use rand::Rng;

use common::*;
use arena::ArenaPlugin;
use paddle::{PaddlePlugin, PaddleMarker, AllPaddleHitboxes, PaddleHitbox};
use score::{ScorePlugin, PlayerScored, MaxScoreReached, ClearScores};

pub struct PongPlugin;

impl Plugin for PongPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_plugins(DefaultPlugins.set(
                WindowPlugin {
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
                })
            )
            .add_plugins(ArenaPlugin)
            .add_plugins(PaddlePlugin)
            .add_plugins(ScorePlugin)
            .insert_resource(RoundStartTimer::default())
            .insert_resource(GameState::default())
            .add_systems(Startup, setup_ball)
            .add_systems(PostStartup, start_round_timer)
            .add_systems(Update, handle_game_end)
            .add_systems(Update, move_ball.after(paddle::Systems::HandleInput))
            .add_systems(Update, update_round_timer.before(score::Systems::Update))
            .add_systems(Update, detect_score.after(move_ball).before(score::Systems::Update))
            .configure_sets(Startup, (arena::Systems::CameraSetup).before(score::Systems::Startup));
    }
}

#[derive(Component)]
struct Ball {
    movement_dir: Dir2,
    paused: bool,
}

impl Default for Ball {
    fn default() -> Self {
        Ball { movement_dir: Dir2::X, paused: true }
    }
}

fn setup_ball(
    mut commands: Commands
) {
    commands.spawn((
        Ball::default(),
        Sprite {
            color: Color::srgb_u8(0, 255, 0),
            custom_size: Some(Vec2::ONE),
            anchor: Anchor::Center,
            ..default()
        },
        Transform::from_scale(
            Vec2::splat(BALL_SIZE_AS_SCREEN_HEIGHT_PCT * ARENA_HEIGHT).extend(0f32)
        ),
    ));
}

#[derive(Resource, Default)]
struct RoundStartTimer(Timer);

#[derive(Resource, Default)]
struct GameState {
    between_games: bool,
}

fn start_round_timer(mut round_timer: ResMut<RoundStartTimer>) {
    round_timer.0 = Timer::from_seconds(2f32, TimerMode::Once);
}

fn update_round_timer(
    mut round_timer: ResMut<RoundStartTimer>,
    time: Res<Time>,
    mut game_state: ResMut<GameState>,
    mut ball: Single<&mut Ball>,
    mut event_writer: EventWriter<ClearScores>,
) {
    round_timer.0.tick(time.delta());

    if round_timer.0.just_finished() {

        // Reset for new game if needed
        if game_state.between_games {
            game_state.between_games = false;
            event_writer.write(ClearScores);
        }

        // Generate a random starting angle (w/ 50% change of each direction)
        let mut rng = rand::rng();
        let random_angle = rng.random_range(-(PI/7f32)..(PI/7f32));
        let mut rotation_quat = Quat::from_rotation_z(random_angle);
        if rng.random_bool(1.0 / 2.0) {
            // flip rotation 180 degrees
            rotation_quat *= Quat::from_rotation_z(PI);
        }

        ball.movement_dir = Dir2::new_unchecked((rotation_quat * Vec3::X).xy());
        ball.paused = false;
    }
}

// Collides the ball once with the nearest surface (wall or paddle). This function
// will move the ball to the collision point and update its movement vector.
// If a collision occurred, Some(f32) will be returned with the distance that
// the ball has moved to reach this collision point. None is returned for no
// collision. Ideally, this function should be called repeatedly until None is returned
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
        (Vec2::new(0f32, (ARENA_HEIGHT / 2f32) - ball_rad), Plane2d::new(Vec2::NEG_Y))
    } else {
        // Otherwise, bottom wall
        (Vec2::new(0f32, (-ARENA_HEIGHT / 2f32) + ball_rad), Plane2d::new(Vec2::Y))
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
                paddle_collision = Some(Collision(
                    dist, paddle.1, Some(impact_point)
                ));
            }
        }
    }

    let mut wall_collision: Option<Collision> = None;
    if let Some(dist) = ball_ray.intersect_plane(wall.0, wall.1) {
        if dist <= move_dist {
            wall_collision = Some(Collision(
                dist, wall.1, None
            ));
        }
    }

    let mut apply_collision = |collision: Collision| {
        let impact_point = collision.2.unwrap_or(ball_ray.get_point(collision.0));
        ball_tf.translation = impact_point.extend(0f32);
        ball.movement_dir = Dir2::new_unchecked(
            ball.movement_dir.reflect(collision.1.normal.as_vec2())
        );
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

fn move_ball(
    time: Res<Time>,
    ball_q: Single<(&mut Ball, &mut Transform), Without<PaddleMarker>>,
    paddles: Query<AllPaddleHitboxes>,
) {
    let (mut ball, mut ball_tf) = ball_q.into_inner();

    if !ball.paused {
        let mut move_dist = time.delta_secs() * BALL_MOVE_SPEED;
        loop {
            let collision_dist = collide_once(
                move_dist, &mut ball, &mut ball_tf, paddles
            );
            match collision_dist {
                Some(dist) => move_dist -= dist,
                None => break,
            };
        };

        let movement_vec = ball.movement_dir * move_dist;
        ball_tf.translation += movement_vec.extend(0f32);
    }
}

fn detect_score(
    ball_q: Single<(&mut Ball, &mut Transform)>,
    mut round_timer: ResMut<RoundStartTimer>,
    mut event_writer: EventWriter<PlayerScored>,
) {
    let (mut ball, mut ball_tf) = ball_q.into_inner();

    if ball.paused {
        return;
    }

    if ball_tf.translation.x.abs() >= (ARENA_WIDTH / 2f32) {
        // Ball has collided with wall! Raise event about who scored...
        event_writer.write(PlayerScored(
            if ball_tf.translation.x.is_sign_positive() { Player1 } else { Player2 }
        ));

        ball.paused = true;
        ball_tf.translation = Vec3::ZERO;
        round_timer.0 = Timer::from_seconds(1f32, TimerMode::Once);
    }
}

fn handle_game_end(
    mut events: EventReader<MaxScoreReached>,
    mut round_timer: ResMut<RoundStartTimer>,
    mut game_state: ResMut<GameState>,
) {
    if !events.is_empty() {
        events.clear();
        game_state.between_games = true;
        round_timer.0 = Timer::from_seconds(3f32, TimerMode::Once);
    }
}
