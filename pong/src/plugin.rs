use bevy::asset::RenderAssetUsages;
use bevy::sprite::Anchor;
use bevy::text::FontSmoothing;
use bevy::window::WindowResized;
use bevy::render::camera::ScalingMode;
use bevy::render::mesh::PrimitiveTopology;
use bevy::render::mesh::Indices;
use bevy::prelude::*;
use std::f32::consts::PI;
use rand::Rng;

use super::consts::*;

pub struct PongPlugin;

impl Plugin for PongPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (setup_camera, setup_arena, setup_paddles, setup_ball))
            .add_systems(Startup, setup_text_entities)
            .add_systems(PostStartup, start_round_timer)
            .add_systems(Update, (update_round_timer, handle_user_input))
            .add_systems(Update, (handle_window_resize, handle_font_resize))
            .add_systems(Update, move_ball.after(handle_user_input))
            .add_systems(Update, detect_score.after(move_ball))
            .insert_resource(RoundStartTimer::default())
            .insert_resource(ResizeFontDebounce::default())
            .insert_resource(Score {p1: 0, p2: 0});
    }
}

// Sets up the 2D camera for the Pong World
fn setup_camera(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: ScalingMode::AutoMin { min_width: ARENA_WIDTH, min_height: ARENA_HEIGHT },
            ..OrthographicProjection::default_2d()
        })
    ));
}

// Sets up the "Arena" that the game is played within
fn setup_arena(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    // Background black box to outline playing field
    commands.spawn((
        Mesh2d(meshes.add(Rectangle::from_size(Vec2::new(ARENA_WIDTH, ARENA_HEIGHT)))),
        MeshMaterial2d(materials.add(ColorMaterial::from_color(Color::BLACK))),
        Transform::from_translation(Vec3 {
            z: -2f32,
            ..default()
        }),
    ));

    // Dashed line down the middle to separate left and right side of arena
    commands.spawn((
        Mesh2d(create_midline_mesh(&mut meshes)),
        MeshMaterial2d(materials.add(ColorMaterial::from_color(Color::WHITE))),
        Transform::from_translation(Vec3::new(0f32, 0f32, -1f32)),
    ));
}

fn create_midline_mesh(meshes: &mut Assets<Mesh>) -> Handle<Mesh> {
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    );

    let mut vertices: Vec<[f32; 3]> = Vec::new();

    // Push vertices for initial centered dash
    vertices.push([-MIDLINE_DASH_WIDTH / 2f32, MIDLINE_DASH_HEIGHT / 2f32, 0.0]);
    vertices.push([MIDLINE_DASH_WIDTH / 2f32, MIDLINE_DASH_HEIGHT / 2f32, 0.0]);
    vertices.push([MIDLINE_DASH_WIDTH / 2f32, -MIDLINE_DASH_HEIGHT / 2f32, 0.0]);
    vertices.push([-MIDLINE_DASH_WIDTH / 2f32, -MIDLINE_DASH_HEIGHT / 2f32, 0.0]);

    let mut start_y = MIDLINE_DASH_HEIGHT * 1.5f32;

    loop {        
        if start_y >= (ARENA_HEIGHT / 2f32) {
            // This dash would start beyond height of arena. We're done.
            break;
        }

        let end_y = (start_y + MIDLINE_DASH_HEIGHT).min(ARENA_HEIGHT / 2f32);

        // Dash in positive y space
        vertices.push([-MIDLINE_DASH_WIDTH / 2f32, end_y, 0.0]);
        vertices.push([MIDLINE_DASH_WIDTH / 2f32, end_y, 0.0]);
        vertices.push([MIDLINE_DASH_WIDTH / 2f32, start_y, 0.0]);
        vertices.push([-MIDLINE_DASH_WIDTH / 2f32, start_y, 0.0]);

        // Mirrored dash in negative y space
        vertices.push([-MIDLINE_DASH_WIDTH / 2f32, -start_y, 0.0]);
        vertices.push([MIDLINE_DASH_WIDTH / 2f32, -start_y, 0.0]);
        vertices.push([MIDLINE_DASH_WIDTH / 2f32, -end_y, 0.0]);
        vertices.push([-MIDLINE_DASH_WIDTH / 2f32, -end_y, 0.0]);

        start_y = end_y + MIDLINE_DASH_HEIGHT;
    }

    assert!((vertices.len() % 4) == 0, "Error generating midline mesh");

    let mut indices: Vec<u16> = Vec::new();
    for index in 0..(vertices.len() / 4) {
        let i = index * 4;

        // Each dash is 2 triangles within the mesh
        indices.extend_from_slice(&[ i as u16, i as u16 + 1, i as u16 + 2 ]);
        indices.extend_from_slice(&[ i as u16, i as u16 + 2, i as u16 + 3 ]);
    }

    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);
    mesh.insert_indices(Indices::U16(indices));
    meshes.add(mesh)
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

#[derive(Component)]
struct P1ScoreText;

#[derive(Component)]
struct P2ScoreText;

#[derive(Component)]
struct P1WinText;

#[derive(Component)]
struct P2WinText;

#[derive(Component)]
struct DynamicFontSize {
    pct_win_height: f32,
}

fn setup_text_entities(mut commands: Commands, window: Single<&Window>) {

    let score_y = ARENA_HEIGHT / 2f32;
    let win_y = score_y - (SCORE_FONT_SIZE_AS_SCREEN_PCT * ARENA_HEIGHT * 1.1f32);

    commands.spawn((
        P1ScoreText,
        DynamicFontSize { pct_win_height: SCORE_FONT_SIZE_AS_SCREEN_PCT },
        Text2d::new("0"),
        TextFont {
            font_size: SCORE_FONT_SIZE_AS_SCREEN_PCT * window.height(),
            ..default()
        },
        Anchor::TopCenter,
        Transform {
            translation: Vec3::new(-ARENA_WIDTH / 4f32, score_y, -1f32),
            scale: Vec3::splat(ARENA_HEIGHT / window.height()),
            ..default()
        },
    ));

    commands.spawn((
        P2ScoreText,
        DynamicFontSize { pct_win_height: SCORE_FONT_SIZE_AS_SCREEN_PCT },
        Text2d::new("0"),
        TextFont {
            font_size: SCORE_FONT_SIZE_AS_SCREEN_PCT * window.height(),
            ..default()
        },
        Anchor::TopCenter,
        Transform {
            translation: Vec3::new(ARENA_WIDTH / 4f32, score_y, -1f32),
            scale: Vec3::splat(ARENA_HEIGHT / window.height()),
            ..default()
        },
    ));

    commands.spawn((
        P1WinText,
        DynamicFontSize { pct_win_height: WIN_FONT_SIZE_AS_SCREEN_PCT },
        Text2d::new("Player 1 Wins!"),
        TextFont {
            font_size: WIN_FONT_SIZE_AS_SCREEN_PCT * window.height(),
            font_smoothing: FontSmoothing::AntiAliased,
            ..default()
        },
        Anchor::TopCenter,
        Transform {
            translation: Vec3::new(-ARENA_WIDTH / 4f32, win_y, -1f32),
            scale: Vec3::splat(ARENA_HEIGHT / window.height()),
            ..default()
        },
        Visibility::Hidden,
    ));

    commands.spawn((
        P2WinText,
        DynamicFontSize { pct_win_height: WIN_FONT_SIZE_AS_SCREEN_PCT },
        Text2d::new("Player 2 Wins!"),
        TextFont {
            font_size: WIN_FONT_SIZE_AS_SCREEN_PCT * window.height(),
            font_smoothing: FontSmoothing::AntiAliased,
            ..default()
        },
        Anchor::TopCenter,
        Transform {
            translation: Vec3::new(ARENA_WIDTH / 4f32, win_y, -1f32),
            scale: Vec3::splat(ARENA_HEIGHT / window.height()),
            ..default()
        },
        Visibility::Hidden,
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

fn start_round_timer(mut round_timer: ResMut<RoundStartTimer>) {
    round_timer.0 = Timer::from_seconds(2f32, TimerMode::Once);
}

fn update_round_timer(
    mut round_timer: ResMut<RoundStartTimer>,
    time: Res<Time>,
    mut ball: Single<&mut Ball>,
    mut scores: ResMut<Score>,
    p1_score_txt: Single<&mut Text2d, (With<P1ScoreText>, Without<P2ScoreText>)>,
    p2_score_txt: Single<&mut Text2d, (With<P2ScoreText>, Without<P1ScoreText>)>,
    p1_win_txt: Single<&mut Visibility, (With<P1WinText>, Without<P2WinText>)>,
    p2_win_txt: Single<&mut Visibility, (With<P2WinText>, Without<P1WinText>)>,
) {
    round_timer.0.tick(time.delta());

    if round_timer.0.just_finished() {

        // Reset for new game if needed
        if (scores.p1 >= WINNING_SCORE) || (scores.p2 >= WINNING_SCORE) {
            *scores = Score { p1: 0, p2: 0 };
            p1_score_txt.into_inner().0 = String::from("0");
            p2_score_txt.into_inner().0 = String::from("0");
            *p1_win_txt.into_inner() = Visibility::Hidden;
            *p2_win_txt.into_inner() = Visibility::Hidden;
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

#[derive(Resource)]
struct Score {
    p1: u8,
    p2: u8,
}

fn detect_score(
    ball_q: Single<(&mut Ball, &mut Transform)>,
    mut round_timer: ResMut<RoundStartTimer>,
    mut scores: ResMut<Score>,
    p1_score_txt: Single<&mut Text2d, (With<P1ScoreText>, Without<P2ScoreText>)>,
    p2_score_txt: Single<&mut Text2d, (With<P2ScoreText>, Without<P1ScoreText>)>,
    p1_win_txt: Single<&mut Visibility, (With<P1WinText>, Without<P2WinText>)>,
    p2_win_txt: Single<&mut Visibility, (With<P2WinText>, Without<P1WinText>)>,
) {
    let (mut ball, mut ball_tf) = ball_q.into_inner();

    if ball.paused {
        return;
    }

    if ball_tf.translation.x.abs() >= (ARENA_WIDTH / 2f32) {
        // Ball has collided with wall! Check who scored...        
        if ball_tf.translation.x.is_sign_positive() {
            scores.p1 += 1;
            p1_score_txt.into_inner().0 = scores.p1.to_string();
        } else {
            scores.p2 += 1;
            p2_score_txt.into_inner().0 = scores.p2.to_string();
        }

        ball.paused = true;
        ball_tf.translation = Vec3::ZERO;
    }

    if (scores.p1 < WINNING_SCORE) && (scores.p2 < WINNING_SCORE) {
        // No Winner Yet
        round_timer.0 = Timer::from_seconds(1f32, TimerMode::Once);
    } else {
        round_timer.0 = Timer::from_seconds(2f32, TimerMode::Once);
        if scores.p1 > scores.p2 {
            // Player 1 Wins
            *p1_win_txt.into_inner() = Visibility::Visible;
        } else {
            // Player 2 Wins
            *p2_win_txt.into_inner() = Visibility::Visible;
        }
    }
}

#[derive(Resource, Default)]
struct ResizeFontDebounce(Timer);

fn handle_window_resize(
    mut events: EventReader<WindowResized>,
    mut font_timer: ResMut<ResizeFontDebounce>,
) {
    if !events.is_empty() {
        events.clear();
        font_timer.0 = Timer::from_seconds(FONT_RESIZE_DEBOUNCE_TIME, TimerMode::Once);
    }
}

fn handle_font_resize(
    time: Res<Time>,
    mut font_timer: ResMut<ResizeFontDebounce>,
    window: Single<&Window>,
    fonts: Query<(&DynamicFontSize, &mut TextFont, &mut Transform)>,
) {
    //println!("{}", 1f32 / time.delta_secs());
    font_timer.0.tick(time.delta());

    if font_timer.0.just_finished() {
        for (font_cfg, mut font, mut transform) in fonts {
            font.font_size = font_cfg.pct_win_height * window.height();
            transform.scale = Vec3::splat(ARENA_HEIGHT / window.height());
        }
    }
}
