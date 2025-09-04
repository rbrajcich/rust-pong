mod arena;
mod common;
mod score;

use bevy::sprite::Anchor;
use bevy::window::PresentMode;
use bevy::window::WindowResolution;
use bevy::prelude::*;
use std::f32::consts::PI;
use rand::Rng;

use common::*;
use arena::ArenaPlugin;
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
            .add_plugins(ScorePlugin)
            .insert_resource(RoundStartTimer::default())
            .insert_resource(GameState::default())
            .add_systems(Startup, (setup_paddles, setup_ball))
            .add_systems(PostStartup, start_round_timer)
            .add_systems(Update, (handle_user_input, handle_game_end))
            .add_systems(Update, move_ball.after(handle_user_input))
            .add_systems(Update, update_round_timer.before(score::Systems::Update))
            .add_systems(Update, detect_score.after(move_ball).before(score::Systems::Update))
            .configure_sets(Startup, (arena::Systems::CameraSetup).before(score::Systems::Startup));
    }
}

#[derive(Component)]
struct Player1Paddle;

#[derive(Component)]
struct Player2Paddle;

#[derive(Component)]
struct Paddle;

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

fn setup_paddles(mut commands: Commands) {
    let mut paddle_size = Vec2 {
        x: PADDLE_ASPECT_RATIO,
        y: 1f32,
    };
    paddle_size *= ARENA_HEIGHT * PADDLE_HEIGHT_AS_SCREEN_PCT;

    commands.spawn((
        Paddle,
        Player1Paddle,
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
                z: 1.0f32,
            },
            scale: paddle_size.extend(0f32),
            ..default()
        },
    ));

    commands.spawn((
        Paddle,
        Player2Paddle,
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
                z: 1.0f32,
            },
            scale: paddle_size.extend(0f32),
            ..default()
        },
    ));
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

fn handle_user_input(
    player1: Option<Single<&mut Transform, (With<Player1Paddle>, Without<Player2Paddle>)>>,
    player2: Option<Single<&mut Transform, (With<Player2Paddle>, Without<Player1Paddle>)>>,
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
) {
    let mut player1_movement = 0;
    let mut player2_movement = 0;
    let multiplier = time.delta_secs() * ARENA_HEIGHT * 1.5;

    if keys.pressed(KeyCode::KeyW) {
        player1_movement += 1;
    }
    if keys.pressed(KeyCode::KeyS) {
        player1_movement -= 1;
    }
    if keys.pressed(KeyCode::ArrowUp) {
        player2_movement += 1;
    }
    if keys.pressed(KeyCode::ArrowDown) {
        player2_movement -= 1;
    }

    if let Some(mut p1_trans) = player1 {
        let clamp_y = (ARENA_HEIGHT / 2f32) - (p1_trans.scale.y / 2f32);
        let new_y = p1_trans.translation.y + (player1_movement as f32 * multiplier);
        p1_trans.translation.y = new_y.clamp(-clamp_y, clamp_y);
    }

    if let Some(mut p2_trans) = player2 {
        let clamp_y = (ARENA_HEIGHT / 2f32) - (p2_trans.scale.y / 2f32);
        let new_y = p2_trans.translation.y + (player2_movement as f32 * multiplier);
        p2_trans.translation.y = new_y.clamp(-clamp_y, clamp_y);
    }
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
    p1_tf: &Transform,
    p2_tf: &Transform,
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
    //     Paddle top offset for ball size,
    //     Paddle bot offset for ball size
    // )
    let paddle = if ball.movement_dir.x > 0f32 {
        // Focus on collisions with p2 paddle if moving right
        (
            Vec2::new(
                p2_tf.translation.x - (p2_tf.scale.x + ball_rad),
                p2_tf.translation.y,
            ),
            Plane2d::new(Vec2::NEG_X),
            (p2_tf.translation.y - (p2_tf.scale.y / 2f32)) - ball_rad,
            (p2_tf.translation.y + (p2_tf.scale.y / 2f32)) + ball_rad,
        )
    } else {
        // Otherwise, focus on p1 paddle
        (
            Vec2::new(
                p1_tf.translation.x + p1_tf.scale.x + ball_rad,
                p1_tf.translation.y,
            ),
            Plane2d::new(Vec2::X),
            (p1_tf.translation.y - (p1_tf.scale.y / 2f32)) - ball_rad,
            (p1_tf.translation.y + (p1_tf.scale.y / 2f32)) + ball_rad,
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
    ball_q: Single<(&mut Ball, &mut Transform), (Without<Player1Paddle>, Without<Player2Paddle>)>,
    player1: Single<&Transform, With<Player1Paddle>>,
    player2: Single<&Transform, With<Player2Paddle>>,
) {
    let (mut ball, mut ball_tf) = ball_q.into_inner();

    if !ball.paused {
        let mut move_dist = time.delta_secs() * BALL_MOVE_SPEED;

        loop {
            let collision_dist = collide_once(
                move_dist, &mut ball, &mut ball_tf, &player1, &player2
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
