mod common;
mod plugin;

use bevy::app::PluginGroupBuilder;
use bevy::prelude::*;
use bevy::window::PresentMode;
use bevy::window::WindowResolution;

use common::*;
use plugin::PongPlugin;

/// The PluginGroup which contains all plugins needed to run Pong,
/// including the DefaultPlugins
pub struct PongPlugins;

impl PluginGroup for PongPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add_group(DefaultPlugins)
            .set(WindowPlugin {
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
            // .set(RenderPlugin {
            //     render_creation: RenderCreation::from(WgpuSettings {
            //         backends: Some(Backends::DX12),
            //         ..default()
            //     }),
            //     ..default()
            // })
            .add(PongPlugin)
    }
}
