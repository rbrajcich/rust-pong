use bevy::prelude::*;
use bevy::sprite::Anchor;
use bevy::text::FontSmoothing;

use bevy_dyn_fontsize::{DynamicFontsizePlugin, DynamicFontSize};

use crate::common::*;

#[derive(Resource, Default)]
pub struct Score {
    p1: u8,
    p2: u8,
}

#[derive(Event)]
pub struct PlayerScored(pub PlayerId);

#[derive(Event)]
pub struct MaxScoreReached;

#[derive(Event)]
pub struct ClearScores;

pub fn setup(
    mut commands: Commands,
    window: Single<&Window>,
    camera_entity: Single<Entity, With<Camera2d>>,
) {

    let score_y = ARENA_HEIGHT / 2f32;
    let win_y = score_y - (SCORE_FONT_SIZE_AS_SCREEN_PCT * ARENA_HEIGHT * 1.1f32);

    commands.spawn((
        P1ScoreText,
        DynamicFontSize { 
            height_in_world: SCORE_FONT_SIZE_AS_SCREEN_PCT * ARENA_HEIGHT,
            render_camera: camera_entity.entity(),
        },
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
        DynamicFontSize {
            height_in_world: SCORE_FONT_SIZE_AS_SCREEN_PCT * ARENA_HEIGHT,
            render_camera: camera_entity.entity(),
        },
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
        DynamicFontSize {
            height_in_world: WIN_FONT_SIZE_AS_SCREEN_PCT * ARENA_HEIGHT,
            render_camera: camera_entity.entity(),
        },
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
        DynamicFontSize {
            height_in_world: WIN_FONT_SIZE_AS_SCREEN_PCT * ARENA_HEIGHT,
            render_camera: camera_entity.entity(),
        },
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

pub fn handle_player_score(
    mut events: EventReader<PlayerScored>,
    mut scores: ResMut<Score>,
    mut p1_score_txt: Single<&mut Text2d, (With<P1ScoreText>, Without<P2ScoreText>)>,
    mut p2_score_txt: Single<&mut Text2d, (With<P2ScoreText>, Without<P1ScoreText>)>,
    mut p1_win_txt: Single<&mut Visibility, (With<P1WinText>, Without<P2WinText>)>,
    mut p2_win_txt: Single<&mut Visibility, (With<P2WinText>, Without<P1WinText>)>,
    mut event_writer: EventWriter<MaxScoreReached>,
) {
    for PlayerScored(scorer) in events.read() {
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

        if scores.p1 >= WINNING_SCORE {
            event_writer.write(MaxScoreReached);
            *p1_win_txt.as_mut() = Visibility::Visible;
            break;
        } else if scores.p2 >= WINNING_SCORE {
            event_writer.write(MaxScoreReached);
            *p2_win_txt.as_mut() = Visibility::Visible;
            break;
        }
    }
}

pub fn clear_scores(
    mut events: EventReader<ClearScores>,
    mut scores: ResMut<Score>,
    p1_score_txt: Single<&mut Text2d, (With<P1ScoreText>, Without<P2ScoreText>)>,
    p2_score_txt: Single<&mut Text2d, (With<P2ScoreText>, Without<P1ScoreText>)>,
    p1_win_txt: Single<&mut Visibility, (With<P1WinText>, Without<P2WinText>)>,
    p2_win_txt: Single<&mut Visibility, (With<P2WinText>, Without<P1WinText>)>,
) {
    if !events.is_empty() {
        events.clear();

        *scores = Score { p1: 0, p2: 0 };
        p1_score_txt.into_inner().0 = String::from("0");
        p2_score_txt.into_inner().0 = String::from("0");
        *p1_win_txt.into_inner() = Visibility::Hidden;
        *p2_win_txt.into_inner() = Visibility::Hidden;
    }
}

#[derive(Component)]
pub struct P1ScoreText;

#[derive(Component)]
pub struct P2ScoreText;

#[derive(Component)]
pub struct P1WinText;

#[derive(Component)]
pub struct P2WinText;
