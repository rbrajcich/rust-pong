//!
//! The bevy_dyn_fontsize crate contains a plugin allowing pixel-perfect
//! text to be displayed even when the window is resized. It allows this
//! to be done while keeping the text a constant size in world units.
//!
//! Current Limitation: the plugin only adjusts font sizing parameters when
//! the window itself is resized. If the camera projection is altered in some
//! other way, the font size will not be updated. I.e. this plugin assumes
//! a constant camera projection is present.
//!

// -------------------------------------------------------------------------------------------------
// Included Symbols

use std::time::Duration;

use bevy::prelude::*;
use bevy::window::WindowResized;

// -------------------------------------------------------------------------------------------------
// Constants

const DEFAULT_DEBOUNCE_DURATION: Duration = Duration::from_millis(100);

// -------------------------------------------------------------------------------------------------
// Public API

///
/// This plugin allows pixel-perfect text to be displayed with a constant in-world
/// size. If the window is resized, resulting in a change to the ratio of in-world
/// units to on-screen pixels, the plugin will automatically update font rendering
/// to ensure it is pixel-perfect given the new projection.
///
/// It uses a debouncer to avoid storms of updates on every frame while a window
/// is actively being resized.
///
pub struct DynamicFontsizePlugin {
    /// The duration to use when debouncing window resize events. Defaults to 100 ms.
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

///
/// This component should be added to text elements that need to be dynamically
/// sized. It requires a Text2d component on the same entity.
///
#[derive(Component)]
#[require(Text2d)]
pub struct DynamicFontSize {
    /// Font size (height) in world units. The plugin will pin the text to this size in the world.
    pub height_in_world: f32,
    /// The 2D camera rendering this text entity. Dynamic resizing is based on its projection.
    pub render_camera: Entity,
}

// -------------------------------------------------------------------------------------------------
// Private Resources

//
// This resource is added as a core piece of the plugin when it is added to the app.
// It tracks the timing of window resize messages and debounces them.
//
#[derive(Resource)]
struct WindowResizeDebouncer {
    //
    // The debounce timer. It will be "running" after a window resize until
    // the debounce duration has elapsed, and then trigger resizing of text entities.
    //
    timer: Timer,

    // The debounce duration to use after a window resize is detected.
    duration: Duration,
}

impl WindowResizeDebouncer {
    fn from_duration(duration: Duration) -> Self {
        Self {
            timer: Timer::default(), // Don't start timer until a window resize occurs
            duration,
        }
    }
}

// -------------------------------------------------------------------------------------------------
// Private Systems

//
// This system detects all window resize messages. Any time a resize happens, the timer
// is started for the debounce duration. If further resizes occur before the timer
// expires, it will be reset to the debounce duration again. Text resizing will not occur
// until the debounce duration is complete without hitting any more resizes along the way.
//
fn handle_window_resize(
    mut messages: MessageReader<WindowResized>,
    mut debouncer: ResMut<WindowResizeDebouncer>,
) {
    if !messages.is_empty() {
        messages.clear();
        debouncer.timer = Timer::new(debouncer.duration, TimerMode::Once);
    }
}

//
// This system is responsible for the actual resizing of relevant text entities. It only
// provides this functionality if the debounce timer has just completed. Note that
// adjustments are only performed on entities with DynamicFontSize components.
// The system will use the camera's projection to detect the new on-screen size of
// the text, and set its font size and scale accordingly.
//
// Note: if the render camera is invalid or doesn't use an orthographic projection,
// no text sizing adjustments will be performed for that entity.
//
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

        // projection.area includes entire panel of window in world units, even
        // if there are borders or cropped out bits.
        let cam_height = projection.area.height();
        let win_height = window.height();

        // Skip on 0 to cover "minimize" case and prevent divide-by-zero scenario
        if (cam_height > 0f32) && (win_height > 0f32) {
            // win_height / cam_height gives us conversion b/t in-world and pixel units
            font.font_size = (font_cfg.height_in_world / cam_height) * win_height;
            transform.scale = Vec3::splat(cam_height / win_height);
        }
    }
}

// -------------------------------------------------------------------------------------------------
// Unit Tests

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::window::WindowResolution;

    #[test]
    fn test_plugin_build() {
        let mut app = App::new();
        app.add_plugins(DynamicFontsizePlugin::default());

        // Validate WindowResizeDebouncer is created appropriately by plugin build
        let world = app.world();
        match world.get_resource::<WindowResizeDebouncer>() {
            None => panic!(
                "Expected WindowResizeDebouncer resource to be added by DynamicFontsizePlugin"
            ),
            Some(debouncer) => {
                assert_eq!(
                    debouncer.duration, DEFAULT_DEBOUNCE_DURATION,
                    "Expected default debounce duration in plugin"
                );
                assert_eq!(
                    debouncer.timer,
                    Timer::default(),
                    "Expected default timer in newly-created debouncer",
                );
            }
        }

        // Validate systems were added to Update schedule as intended
        let mut exp_update_systems = [
            (core::any::type_name_of_val(&handle_window_resize), false),
            (core::any::type_name_of_val(&handle_font_resize), false),
        ];
        app.get_schedule(Update)
            .expect("Expected Update schedule to exist in app")
            .graph()
            .systems
            .iter()
            .for_each(|(_, boxed_sys, _)| {
                for exp_sys in exp_update_systems.iter_mut() {
                    if boxed_sys.name().as_string() == exp_sys.0 {
                        assert!(
                            !exp_sys.1,
                            "Expected to find {} only once in Update, but found twice",
                            exp_sys.0,
                        );
                        exp_sys.1 = true;
                        return;
                    }
                }
            });
        for exp_sys in exp_update_systems {
            assert!(
                exp_sys.1,
                "Expected to find {} in Update schedule, but it was missing",
                exp_sys.0,
            );
        }
    }

    #[test]
    fn test_plugin_build_nondefault() {
        let mut app = App::new();
        app.add_plugins(DynamicFontsizePlugin {
            debounce_time: Duration::from_secs(4),
        });

        // Validate WindowResizeDebouncer is created appropriately by plugin build
        let world = app.world();
        match world.get_resource::<WindowResizeDebouncer>() {
            None => panic!(
                "Expected WindowResizeDebouncer resource to be added by DynamicFontsizePlugin"
            ),
            Some(debouncer) => {
                assert_eq!(
                    debouncer.duration,
                    Duration::from_secs(4),
                    "Expected custom debounce duration from plugin cfg to be in resource"
                );
            }
        }
    }

    #[test]
    fn test_handle_window_resize_system() {
        let mut world = World::default();

        // Register our system with the world, plus some resources it needs.
        // We simulate a partially elapsed timer in place already.
        let resize_sys = world.register_system(handle_window_resize);
        world.init_resource::<Messages<WindowResized>>();
        let mut inflight_timer = Timer::new(Duration::from_secs(1), TimerMode::Once);
        inflight_timer.tick(Duration::from_millis(500));
        world.insert_resource(WindowResizeDebouncer {
            duration: Duration::from_secs(1),
            timer: inflight_timer.clone(),
        });

        // Run the system with no messages in place. Expect nothing to change
        world
            .run_system(resize_sys)
            .expect("Expected resize system to run successfully");
        let debouncer = world.get_resource::<WindowResizeDebouncer>().unwrap();
        assert_eq!(
            debouncer.timer, inflight_timer,
            "Expected no change to timer in debouncer after running sys with no messages",
        );

        // Run the system with a few messages in place. Expect timer to reset
        // and messages to all be consumed by the system
        let msg = WindowResized {
            window: Entity::PLACEHOLDER,
            width: 1920f32,
            height: 1080f32,
        };
        world.write_message(msg.clone());
        world.write_message(msg.clone());
        world.write_message(msg);
        world
            .run_system(resize_sys)
            .expect("Expected resize system to run successfully");
        let mut debouncer = world.get_resource_mut::<WindowResizeDebouncer>().unwrap();
        assert_eq!(
            debouncer.timer,
            Timer::new(Duration::from_secs(1), TimerMode::Once),
            "Expected timer to be reset in debouncer after running sys with messages",
        );

        // Simulate timer elapsed again. Run system again and expect nothing.
        // Last run of system should have consumed all messages.
        debouncer.timer = inflight_timer.clone();
        world
            .run_system(resize_sys)
            .expect("Expected resize system to run successfully");
        let debouncer = world.get_resource::<WindowResizeDebouncer>().unwrap();
        assert_eq!(
            debouncer.timer, inflight_timer,
            "Expected no change to timer in debouncer in final system run with no new messages",
        );
    }

    #[test]
    fn test_handle_font_resize_system() {
        let mut world = World::default();

        // Insert necessary resources. Timer primed to fire in 1 sec
        world.init_resource::<Time>();
        world.insert_resource(WindowResizeDebouncer {
            duration: Duration::from_secs(1),
            timer: Timer::new(Duration::from_secs(1), TimerMode::Once),
        });

        // Local copy of some configured heights, for easier access
        let height_in_world_1 = 4f32;
        let height_in_world_2 = 30f32;
        let win_height = 200;
        let proj_height = 20f32;

        // First, create and run setup system to get Entities in place and store their id's
        let setup_sys = world.register_system(
            // Create a couple text elements for system to act on, plus projection
            move |mut commands: Commands| {
                commands.spawn(Window {
                    resolution: WindowResolution::new(0, 0), // Start with 0 scenario
                    ..default()
                });
                let p_ortho = commands
                    .spawn(Projection::Orthographic(OrthographicProjection {
                        area: Rect::new(0f32, 0f32, 0f32, 0f32), // Start with 0 scenario
                        ..OrthographicProjection::default_2d()
                    }))
                    .id();
                let p_persp = commands
                    .spawn(Projection::Perspective(PerspectiveProjection::default()))
                    .id();
                let txt1 = commands
                    .spawn((DynamicFontSize {
                        height_in_world: height_in_world_1,
                        render_camera: p_ortho,
                    },))
                    .id();
                let txt2 = commands
                    .spawn((DynamicFontSize {
                        height_in_world: height_in_world_2,
                        render_camera: p_ortho,
                    },))
                    .id();
                let txt3 = commands
                    .spawn((DynamicFontSize {
                        height_in_world: 100f32,
                        render_camera: p_persp,
                    },))
                    .id();
                let txt4 = commands
                    .spawn((DynamicFontSize {
                        height_in_world: 100f32,
                        render_camera: Entity::PLACEHOLDER,
                    },))
                    .id();

                // Return each entity id for test to use
                (p_ortho, txt1, txt2, txt3, txt4)
            },
        );
        let (proj, txt1, txt2, txt3, txt4) = world.run_system(setup_sys).unwrap();

        // Register resize_sys, which we are testing
        let resize_sys = world.register_system(handle_font_resize);

        // Simulate some time passed, but not enough to trigger timer.
        // Nothing should change when we run the system yet.
        world
            .get_resource_mut::<Time>()
            .unwrap()
            .advance_by(Duration::from_millis(900));
        world
            .run_system(resize_sys)
            .expect("Expected resize system to succeed on first run (without timer finishing)");
        let mut query = world.query::<(&TextFont, &Transform)>();
        for (idx, txt) in [txt1, txt2, txt3, txt4].iter().enumerate() {
            let (font, transform) = query.get(&world, *txt).unwrap();
            assert_eq!(
                font.font_size,
                TextFont::default().font_size,
                "Expected no change to TextFont {} size after first run (without timer finishing)",
                idx + 1,
            );
            assert_eq!(
                *transform,
                Transform::default(),
                "Expected no change to Transform {} size after first run (without timer finishing)",
                idx + 1,
            );
        }

        // Now trigger timer, but projection height and window height are 0.
        // We still expect no change to the text entity sizing.
        world
            .get_resource_mut::<Time>()
            .unwrap()
            .advance_by(Duration::from_millis(100));
        world
            .run_system(resize_sys)
            .expect("Expected resize system to succeed on second run (zero height case)");
        let mut query = world.query::<(&TextFont, &Transform)>();
        for (idx, txt) in [txt1, txt2, txt3, txt4].iter().enumerate() {
            let (font, transform) = query.get(&world, *txt).unwrap();
            assert_eq!(
                font.font_size,
                TextFont::default().font_size,
                "Expected no change to TextFont {} size after second run (zero height case)",
                idx + 1,
            );
            assert_eq!(
                *transform,
                Transform::default(),
                "Expected no change to Transform {} size after second run (zero height case)",
                idx + 1,
            );
        }

        // Confirm timer fired during last run
        assert!(
            world
                .get_resource::<WindowResizeDebouncer>()
                .unwrap()
                .timer
                .just_finished(),
            "Expected debounce timer to have finished on last tick during resize system",
        );

        // Now prime for real test. Set projection and window size to non-zero, and reset timer.
        let mut win = world.query::<&mut Window>().single_mut(&mut world).unwrap();
        win.resolution = WindowResolution::new(500, win_height);
        let proj = world
            .query::<&mut Projection>()
            .get_mut(&mut world, proj)
            .unwrap();
        if let Projection::Orthographic(ortho) = proj.into_inner() {
            ortho.area = Rect::new(0f32, 0f32, 600f32, proj_height);
        } else {
            panic!();
        }
        world
            .get_resource_mut::<WindowResizeDebouncer>()
            .unwrap()
            .timer = Timer::new(Duration::from_secs(1), TimerMode::Once);
        world
            .get_resource_mut::<Time>()
            .unwrap()
            .advance_by(Duration::from_millis(1100));

        // Trigger the system again. Now we will expect the entities to be updated
        world
            .run_system(resize_sys)
            .expect("Expected resize system to succeed on third run (nominal case)");
        let mut query = world.query::<(&TextFont, &Transform)>();

        // Validate sizing adjustments for first text element
        let (font, transform) = query.get(&world, txt1).unwrap();
        assert_eq!(
            font.font_size,
            (height_in_world_1 / proj_height) * win_height as f32,
            "Expected TextFont 1 to have correctly-adjusted size on third run (nominal case)",
        );
        assert_eq!(
            transform.scale.y,
            proj_height / win_height as f32,
            "Expected Transform 1 to have correctly-adjusted height on third run (nominal case)",
        );
        assert_eq!(
            transform.scale.x, transform.scale.y,
            "Expected Transform 1 to have equal x/y scaling on third run (nominal case)",
        );

        // Validate sizing adjustments for second text element
        let (font, transform) = query.get(&world, txt2).unwrap();
        assert_eq!(
            font.font_size,
            (height_in_world_2 / proj_height) * win_height as f32,
            "Expected TextFont 2 to have correctly-adjusted size on third run (nominal case)",
        );
        assert_eq!(
            transform.scale.y,
            proj_height / win_height as f32,
            "Expected Transform 2 to have correctly-adjusted height on third run (nominal case)",
        );
        assert_eq!(
            transform.scale.x, transform.scale.y,
            "Expected Transform 2 to have equal x/y scaling on third run (nominal case)",
        );

        // Validate remaining text elements (should not have been adjusted due to invalid cfg)
        for (idx, txt) in [txt3, txt4].iter().enumerate() {
            let (font, transform) = query.get(&world, *txt).unwrap();
            assert_eq!(
                font.font_size,
                TextFont::default().font_size,
                "Expected no change to TextFont {} size after third run (nominal case)",
                idx + 3,
            );
            assert_eq!(
                *transform,
                Transform::default(),
                "Expected no change to Transform {} size after third run (nominal case)",
                idx + 3,
            );
        }
    }
}
