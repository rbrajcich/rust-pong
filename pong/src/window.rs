//!
//! The window module contains code to set up the base engine and create the
//! window in which the pong game is played.
//!

// -------------------------------------------------------------------------------------------------
// Included Symbols

use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::Backends;
use bevy::render::settings::RenderCreation;
use bevy::render::settings::WgpuSettings;
use bevy::window::PresentMode;
use bevy::window::WindowMode;
use bevy::window::WindowResolution;

// -------------------------------------------------------------------------------------------------
// Constants

const PONG_WINDOW_TITLE: &str = "Rust Pong";
const INITIAL_WINDOW_WIDTH: u32 = 1600;
const INITIAL_WINDOW_HEIGHT: u32 = 900;
const WINDOW_SIZE_CONSTRAINTS: WindowResizeConstraints = WindowResizeConstraints {
    min_width: 160.0,
    min_height: 90.0,
    max_width: 7680.0,
    max_height: 4320.0,
};
const EXIT_WINDOW_KEY: KeyCode = KeyCode::Escape;
const TOGGLE_VSYNC_KEY: KeyCode = KeyCode::KeyV;
const TOGGLE_FULLSCREEN_KEY: KeyCode = KeyCode::KeyF;

// -------------------------------------------------------------------------------------------------
// Public API

///
/// The PongWindowPlugin is the main type required to be added to the game to implement
/// the window for pong. The plugin will create a new window on the screen configured
/// with default settings. It will also handle keypress events to change window settings
/// or exit the window.
///
pub struct PongWindowPlugin;

impl Plugin for PongWindowPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: PONG_WINDOW_TITLE.to_string(),
                        resize_constraints: WINDOW_SIZE_CONSTRAINTS,
                        present_mode: PresentMode::AutoVsync,
                        resolution: WindowResolution::new(
                            INITIAL_WINDOW_WIDTH,
                            INITIAL_WINDOW_HEIGHT,
                        ),
                        ..default()
                    }),
                    ..default()
                })
                .set(RenderPlugin {
                    render_creation: RenderCreation::Automatic(WgpuSettings {
                        backends: Some(Backends::DX12),
                        ..default()
                    }),
                    ..default()
                }),
        )
        .add_systems(Update, (handle_exit_pressed, update_window_settings));
    }
}

// -------------------------------------------------------------------------------------------------
// Private Systems

// Detects when the exit key is pressed, and gracefully shuts down the window and app
fn handle_exit_pressed(keys: Res<ButtonInput<KeyCode>>, mut exit_msgs: MessageWriter<AppExit>) {
    if keys.just_pressed(EXIT_WINDOW_KEY) {
        exit_msgs.write(AppExit::Success);
    }
}

//
// Detects when the vsync or fullscreen toggle keys are pressed, and toggles the
// corresponding setting on the game window.
//
fn update_window_settings(keys: Res<ButtonInput<KeyCode>>, mut window: Single<&mut Window>) {
    if keys.just_pressed(TOGGLE_VSYNC_KEY) {
        window.present_mode = match window.present_mode {
            PresentMode::AutoVsync => PresentMode::Immediate,
            _ => PresentMode::AutoVsync,
        };
    }

    if keys.just_pressed(TOGGLE_FULLSCREEN_KEY) {
        window.mode = match window.mode {
            WindowMode::Windowed => WindowMode::BorderlessFullscreen(MonitorSelection::Primary),
            _ => WindowMode::Windowed,
        };
    }
}
