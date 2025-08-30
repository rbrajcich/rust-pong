use std::time::Duration;

use bevy::prelude::*;
use bevy::window::WindowResized;

const DEFAULT_DEBOUNCE_DURATION: Duration = Duration::from_millis(100);

pub struct DynamicFontsizePlugin {
    pub debounce_time: Duration,
}

impl Default for DynamicFontsizePlugin {
    fn default() -> Self {
        DynamicFontsizePlugin {
            debounce_time: DEFAULT_DEBOUNCE_DURATION,
        }
    }
}

impl Plugin for DynamicFontsizePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (handle_window_resize, handle_font_resize))
            .insert_resource(WindowResizeDebouncer::from_duration(self.debounce_time));
    }
}

#[derive(Component)]
pub struct DynamicFontSize {
    pub height_in_world: f32,
    pub render_camera: Entity,
}

#[derive(Resource)]
struct WindowResizeDebouncer {
    timer: Timer,
    duration: Duration,
}

impl WindowResizeDebouncer {
    fn from_duration(duration: Duration) -> Self {
        Self {
            timer: Timer::default(),
            duration,
        }
    }
}

fn handle_window_resize(
    mut events: EventReader<WindowResized>,
    mut debouncer: ResMut<WindowResizeDebouncer>,
) {
    if !events.is_empty() {
        events.clear();
        debouncer.timer = Timer::new(debouncer.duration, TimerMode::Once);
    }
}

fn handle_font_resize(
    time: Res<Time>,
    mut debouncer: ResMut<WindowResizeDebouncer>,
    window: Single<&Window>,
    fonts: Query<(&DynamicFontSize, &mut TextFont, &mut Transform)>,
    projections: Query<&Projection>,
) {
    debouncer.timer.tick(time.delta());

    if !debouncer.timer.just_finished() {
        return;
    }

    for (font_cfg, mut font, mut transform) in fonts {
        let projection = projections.get(font_cfg.render_camera);

        let Ok(Projection::Orthographic(projection)) = projection else {
            // If we can't find the associated projection, just leave the sizing.
            return;
        };

        let cam_height = projection.area.height();
        let win_height = window.height();

        if (cam_height > 0f32) && (win_height > 0f32) {
            font.font_size = (font_cfg.height_in_world / cam_height) * win_height;
            transform.scale = Vec3::splat(cam_height / win_height);
        }
    }
}
