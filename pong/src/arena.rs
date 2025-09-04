//!
//! The arena module contains code to set up the environment in which the
//! pong game is played. This includes the gameplay box itself, the dashed
//! line down the middle, and the camera to render the scene.
//!

// -------------------------------------------------------------------------------------------------
// Included Symbols

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::camera::ScalingMode;
use bevy::render::mesh::{Indices, PrimitiveTopology};

use crate::common::*;

// -------------------------------------------------------------------------------------------------
// Constants

pub const MIDLINE_WIDTH_AS_ARENA_WIDTH_PCT: f32 = 0.005;
pub const MIDLINE_HEIGHT_AS_ARENA_HEIGHT_PCT: f32 = 0.055;
pub const MIDLINE_DASH_WIDTH: f32 = MIDLINE_WIDTH_AS_ARENA_WIDTH_PCT * ARENA_WIDTH;
pub const MIDLINE_DASH_HEIGHT: f32 = MIDLINE_HEIGHT_AS_ARENA_HEIGHT_PCT * ARENA_HEIGHT;
pub const MIDLINE_X_MAG: f32 = MIDLINE_DASH_WIDTH / 2f32; // Magnitude of x coords of vertices
pub const MIDLINE_Y_MAX: f32 = ARENA_HEIGHT / 2f32; // Max y coord value, end line here

// -------------------------------------------------------------------------------------------------
// Public API

///
/// The ArenaPlugin is the main type required to be added to the game to implement
/// the environment of pong. The plugin will add a background rectangle of dimensions
/// common::ARENA_WIDTH x ARENA_HEIGHT, a dashed middle line, and a single 2d camera
/// which is used to render the arena and its contents.
///
pub struct ArenaPlugin;

impl Plugin for ArenaPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_camera.in_set(Systems::CameraSetup))
            .add_systems(Startup, setup_arena.in_set(Systems::ArenaSetup));
    }
}

/// These SystemSets are used to control any system ordering dependencies on this plugin
#[derive(SystemSet, Debug, Clone, Hash, PartialEq, Eq)]
pub enum Systems {
    /// CameraSetup will implement all logic to create the 2d camera entity. Must be in Setup.
    CameraSetup,

    ///
    /// ArenaSetup will implement all logic to create the on screen background
    /// rectangle and dashed midline entities. Must be in Setup.
    ///
    ArenaSetup,
}

// -------------------------------------------------------------------------------------------------
// Private Systems

// Sets up the 2D camera focused on the arena in the game world
fn setup_camera(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: ScalingMode::AutoMin {
                min_width: ARENA_WIDTH,
                min_height: ARENA_HEIGHT,
            },
            ..OrthographicProjection::default_2d()
        }),
    ));
}

// Sets up the arena that the game is played in, including the dashed midline
fn setup_arena(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    // Background black box to outline playing arena
    commands.spawn((
        Mesh2d(meshes.add(Rectangle::from_size(Vec2::new(ARENA_WIDTH, ARENA_HEIGHT)))),
        MeshMaterial2d(materials.add(ColorMaterial::from_color(Color::BLACK))),
        Transform::from_translation(Vec3::new(0f32, 0f32, Z_BACKGROUND)),
    ));

    // Dashed line down the middle to separate left and right side of arena
    commands.spawn((
        Mesh2d(add_midline_mesh(&mut meshes)),
        MeshMaterial2d(materials.add(ColorMaterial::from_color(Color::WHITE))),
        Transform::from_translation(Vec3::new(0f32, 0f32, Z_BEHIND_GAMEPLAY)),
    ));
}

// -------------------------------------------------------------------------------------------------
// Private Functions

//
// Generates a mesh for a dashed vertical line whose height is equal to ARENA_HEIGHT
// and adds it to the provided Assets<Mesh>, returning the handle.
//
fn add_midline_mesh(meshes: &mut Assets<Mesh>) -> Handle<Mesh> {
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    );

    // Vertex Vec, each item is a vertex of 3d coordinates [x, y, z]
    let mut vertices: Vec<[f32; 3]> = Vec::new();

    // This closure adds the 4 vertices for a single dash
    let mut add_dash_vertices = |bot_y, top_y| {
        vertices.push([-MIDLINE_X_MAG, top_y, 0.0]); // Top Left
        vertices.push([MIDLINE_X_MAG, top_y, 0.0]); // Top Right
        vertices.push([MIDLINE_X_MAG, bot_y, 0.0]); // Bottom Right
        vertices.push([-MIDLINE_X_MAG, bot_y, 0.0]); // Bottom Left
    };

    // Add initial dash centered vertically
    add_dash_vertices(-MIDLINE_DASH_HEIGHT / 2f32, MIDLINE_DASH_HEIGHT / 2f32);

    // (0.5*height) to skip half of initial dash, + (1.0*height) to leave a blank space
    let mut start_y = MIDLINE_DASH_HEIGHT * 1.5f32;

    // Each iter, create 2 symmetrical top/bottom dashes, moving away from center point
    loop {
        if start_y >= (MIDLINE_Y_MAX) {
            // This dash would start beyond height of arena. We're done.
            break;
        }

        let end_y = (start_y + MIDLINE_DASH_HEIGHT).min(MIDLINE_Y_MAX);

        add_dash_vertices(start_y, end_y);
        add_dash_vertices(-end_y, -start_y);

        start_y = end_y + MIDLINE_DASH_HEIGHT;
    }

    assert_eq!(vertices.len() % 4, 0, "Error generating midline mesh");

    // For each dash (4 vertices), create 2 triangles out of the vertices to "fill" it
    let mut indices: Vec<u16> = Vec::new();
    for index in 0..(vertices.len() / 4) {
        // Let i be the index of the first (top left) vertex in above Vec
        let i = index * 4;

        // Each triangle is 3 vertices, referenced by their index in above Vec
        indices.extend_from_slice(&[i as u16, i as u16 + 1, i as u16 + 2]);
        indices.extend_from_slice(&[i as u16, i as u16 + 2, i as u16 + 3]);
    }

    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);
    mesh.insert_indices(Indices::U16(indices));
    meshes.add(mesh)
}

// -------------------------------------------------------------------------------------------------
// Unit Tests

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::query::QuerySingleError::{MultipleEntities, NoEntities};
    use bevy::ecs::schedule::ScheduleBuildError;

    #[test]
    fn test_plugin_build() {
        let mut app = App::new();
        app.add_plugins(ArenaPlugin);

        // Validate systems were added to Startup schedule as intended
        let mut exp_startup_systems = [
            (core::any::type_name_of_val(&setup_camera), false),
            (core::any::type_name_of_val(&setup_arena), false),
        ];
        app.get_schedule(Startup)
            .expect("Expected Startup schedule to exist in app")
            .graph()
            .systems()
            .for_each(|(_, boxed_sys, _)| {
                for exp_sys in exp_startup_systems.iter_mut() {
                    if boxed_sys.name() == exp_sys.0 {
                        assert!(
                            !exp_sys.1,
                            "Expected to find {} only once in Startup, but found twice",
                            exp_sys.0,
                        );
                        exp_sys.1 = true;
                        return;
                    }
                }
            });
        for exp_sys in exp_startup_systems {
            assert!(
                exp_sys.1,
                "Expected to find {} in Startup schedule, but it was missing",
                exp_sys.0,
            );
        }
    }

    #[test]
    fn test_sys_ordering_camera() {
        let mut app = App::new();
        app.add_plugins(ArenaPlugin);

        // This ordering will lead to an error (which we expect) if the system
        // is in the system set as it should be.
        app.configure_sets(Startup, Systems::CameraSetup.before(setup_camera));
        let init_result = app
            .world_mut()
            .try_schedule_scope(Startup, |world, sched| sched.initialize(world))
            .expect("Expected Startup schedule to exist in app");
        let Err(ScheduleBuildError::SetsHaveOrderButIntersect(..)) = init_result else {
            panic!(concat!(
                "Expected Startup schedule build to fail, ",
                "since 'setup_camera' should be in CameraSetup system set. But it succeeded"
            ));
        };
    }

    #[test]
    fn test_sys_ordering_arena() {
        let mut app = App::new();
        app.add_plugins(ArenaPlugin);

        // This ordering will lead to an error (which we expect) if the system
        // is in the system set as it should be.
        app.configure_sets(Startup, Systems::ArenaSetup.before(setup_arena));
        let init_result = app
            .world_mut()
            .try_schedule_scope(Startup, |world, sched| sched.initialize(world))
            .expect("Expected Startup schedule to exist in app");
        let Err(ScheduleBuildError::SetsHaveOrderButIntersect(..)) = init_result else {
            panic!(concat!(
                "Expected Startup schedule build to fail, ",
                "since 'setup_arena' should be in ArenaSetup system set. But it succeeded"
            ));
        };
    }

    #[test]
    fn test_camera_setup_system() {
        let mut world = World::default();
        let setup_sys = world.register_system(setup_camera);

        // Run the system and validate 1 Camera was created with correct Projection
        world.run_system(setup_sys).unwrap();
        let mut query = world.query_filtered::<&Projection, With<Camera2d>>();
        match query.single(&world) {
            Ok(Projection::Orthographic(proj)) => match proj.scaling_mode {
                ScalingMode::AutoMin {
                    min_width,
                    min_height,
                } => {
                    assert_eq!(
                        min_width, ARENA_WIDTH,
                        "Expected ScalingMode min_width of ARENA_WIDTH, but got {min_width}",
                    );
                    assert_eq!(
                        min_height, ARENA_HEIGHT,
                        "Expected ScalingMode min_height of ARENA_HEIGHT, but got {min_height}",
                    );
                }
                _ => panic!("Expected Scaling Mode AutoMin, got {:?}", proj.scaling_mode),
            },
            Ok(proj) => panic!("Expected Camera with OrthographicProjection, got {proj:?}"),
            Err(NoEntities(_)) => panic!("Expected single Camera, but none found."),
            Err(MultipleEntities(_)) => panic!("Expected single Camera, but found multiple."),
        }
    }
}
