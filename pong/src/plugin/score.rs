use bevy::prelude::*;
use bevy::sprite::Anchor;
use bevy::text::FontSmoothing;

use bevy_dyn_fontsize::{DynamicFontsizePlugin, DynamicFontSize};

use crate::common::*;

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

#[derive(Event)]
pub struct PlayerScored(pub PlayerId);

#[derive(Event)]
pub struct MaxScoreReached;

#[derive(Event)]
pub struct ClearScores;

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum Systems {
    Startup,
    Update,
}

#[derive(Resource, Default)]
struct Score {
    p1: u8,
    p2: u8,
}

#[derive(Component)]
struct ScoreText(PlayerId);

#[derive(Component)]
struct WinText(PlayerId);

fn setup(
    mut commands: Commands,
    window: Single<&Window>,
    camera_entity: Single<Entity, With<Camera2d>>,
) {

    let score_y = ARENA_HEIGHT / 2f32;
    let win_y = score_y - (SCORE_FONT_SIZE_AS_SCREEN_PCT * ARENA_HEIGHT * 1.1f32);

    commands.spawn((
        ScoreText(Player1),
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
        ScoreText(Player2),
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
        WinText(Player1),
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
        WinText(Player2),
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

fn handle_player_score(
    mut events: EventReader<PlayerScored>,
    mut scores: ResMut<Score>,
    mut score_texts: Query<(&mut Text2d, &ScoreText)>,
    mut win_texts: Query<(&mut Visibility, &WinText)>,
    mut event_writer: EventWriter<MaxScoreReached>,
) {
    let mut score_text_iter = score_texts.iter_mut();
    let t1 = score_text_iter.next();
    let t2 = score_text_iter.next();
    assert!(score_text_iter.next().is_none(), "Expected 2 ScoreTexts. Got more");

    let (p1_score_txt, p2_score_txt) = match (t1, t2) {
        (Some(t1), Some(t2)) => {
            if t1.1.0 == Player1 {
                assert!(t2.1.0 == Player2, "Expected Player 2 ScoreText");
                (t1.0.into_inner(), t2.0.into_inner())
            } else {
                assert!(t2.1.0 == Player1, "Expected Player 1 ScoreText");
                (t2.0.into_inner(), t1.0.into_inner())
            }
        },
        _ => panic!("Expected 2 ScoreTexts. Got less")
    };

    let mut win_text_iter = win_texts.iter_mut();
    let t1 = win_text_iter.next();
    let t2 = win_text_iter.next();
    assert!(win_text_iter.next().is_none(), "Expected 2 WinTexts. Got more");

    let (p1_win_txt, p2_win_txt) = match (t1, t2) {
        (Some(t1), Some(t2)) => {
            if t1.1.0 == Player1 {
                assert!(t2.1.0 == Player2, "Expected Player 2 WinText");
                (t1.0.into_inner(), t2.0.into_inner())
            } else {
                assert!(t2.1.0 == Player1, "Expected Player 1 WinText");
                (t2.0.into_inner(), t1.0.into_inner())
            }
        },
        _ => panic!("Expected 2 WinTexts. Got less")
    };

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
            *p1_win_txt = Visibility::Visible;
            break;
        } else if scores.p2 >= WINNING_SCORE {
            event_writer.write(MaxScoreReached);
            *p2_win_txt = Visibility::Visible;
            break;
        }
    }
}

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
