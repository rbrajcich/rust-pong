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
    /// Implements all logic to create the 2d camera entity. Must be in Startup.
    CameraSetup,

    ///
    /// Implements all logic to create the on screen background
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
    use bevy::render::mesh::VertexAttributeValues;

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

    #[test]
    fn test_arena_setup_system() {
        let mut world = World::default();

        // System requires these resources to run
        world.init_resource::<Assets<Mesh>>();
        world.init_resource::<Assets<ColorMaterial>>();

        // Run the system we need to test
        let setup_sys = world.register_system(setup_arena);
        world.run_system(setup_sys).unwrap();

        // Run a helper system to perform validations
        let validate_sys = world.register_system(validate_after_arena_setup);
        world.run_system(validate_sys).unwrap();
    }

    #[test]
    fn test_midline_mesh() {
        let mut meshes = Assets::<Mesh>::default();
        let handle = add_midline_mesh(&mut meshes);
        let mesh = meshes
            .get(handle.id())
            .expect("Expected mesh to be added to meshes asset collection");

        assert_eq!(
            mesh.asset_usage,
            RenderAssetUsages::RENDER_WORLD,
            "Expected midline mesh to only be used by render world",
        );

        assert_eq!(
            mesh.primitive_topology(),
            PrimitiveTopology::TriangleList,
            "Expected midline mesh to use triangle list topology",
        );

        let vals = mesh
            .attribute(Mesh::ATTRIBUTE_POSITION)
            .expect("Expected mesh to contain positional vertex attribute data");

        let VertexAttributeValues::Float32x3(verts) = vals else {
            panic!("Expected positional values to be Float32x3 format");
        };

        let Indices::U16(indices) = mesh.indices().expect("Expected indices in mesh") else {
            panic!("Expected u16 indices for mesh");
        };

        let mut index_chunks = indices.chunks_exact(6);
        assert_eq!(
            index_chunks.remainder().len(),
            0,
            "Expected number of indices in mesh to be divisible by 6",
        );

        // Validate first central dash
        validate_midline_mesh_dash(
            MIDLINE_DASH_HEIGHT / 2f32,
            -MIDLINE_DASH_HEIGHT / 2f32,
            index_chunks
                .next()
                .expect("Expected more dash indices to create dashed line"),
            verts,
        );

        // (0.5*height) to skip half of initial dash, + (1.0*height) to leave a blank space
        let mut start_y = MIDLINE_DASH_HEIGHT * 1.5f32;

        // Each iter, validate 2 symmetrical top/bottom dashes, moving away from center point
        loop {
            if start_y >= (MIDLINE_Y_MAX) {
                // This dash would start beyond height of arena. We're done.
                break;
            }

            let end_y = (start_y + MIDLINE_DASH_HEIGHT).min(MIDLINE_Y_MAX);

            validate_midline_mesh_dash(
                end_y,
                start_y,
                index_chunks
                    .next()
                    .expect("Expected more dash indices to create dashed line"),
                verts,
            );
            validate_midline_mesh_dash(
                -start_y,
                -end_y,
                index_chunks
                    .next()
                    .expect("Expected more dash indices to create dashed line"),
                verts,
            );

            start_y = end_y + MIDLINE_DASH_HEIGHT;
        }
    }

    // --- Helper Functions ---

    fn validate_after_arena_setup(
        color_mat_res: Res<Assets<ColorMaterial>>,
        mesh_res: Res<Assets<Mesh>>,
        query: Query<(&Mesh2d, &MeshMaterial2d<ColorMaterial>)>,
    ) {
        let mut n_entities = 0;
        for (m, mm) in query {
            let color_mat = color_mat_res
                .get(mm.id())
                .expect("Expected to find underlying color material for MeshMaterial2d");
            let mesh = mesh_res
                .get(m.id())
                .expect("Expected to find underlying mesh in Mesh2d");
            match color_mat.color {
                Color::BLACK => {
                    // This must be the background. Sanity check on vertex data
                    let vals = mesh
                        .attribute(Mesh::ATTRIBUTE_POSITION)
                        .expect("Expected mesh to contain positional vertex attribute data");
                    let VertexAttributeValues::Float32x3(vals) = vals else {
                        panic!(
                            "Expected position attr to use Float32x3 format, not {:?}",
                            vals
                        );
                    };
                    let n_vert = vals.len();
                    assert!(
                        n_vert == 4,
                        "Expected 4 vertices in arena background rect. Got {n_vert}",
                    );
                }
                Color::WHITE => {
                    // This must be the dashed line. Verifying its mesh is beyond this test.
                }
                color => panic!("Expected black arena and white background, not {:?}", color),
            }
            n_entities += 1;
        }
        assert!(n_entities == 2, "Expected 2 entities, but got {n_entities}");
    }

    //
    // Check whether a given set of 6 indices contains the necessary vertices/edges to
    // createa valid midline mesh dash between top_y and bot_y.
    //
    fn validate_midline_mesh_dash(top_y: f32, bot_y: f32, indices: &[u16], verts: &Vec<[f32; 3]>) {
        assert_eq!(
            indices.len(),
            6,
            "Expected 6 indices (2 triangles) to make up a dash",
        );

        // Each tuple is an "edge" of a triangle that will be rendered
        let edges = [
            (verts[indices[0] as usize], verts[indices[1] as usize]),
            (verts[indices[1] as usize], verts[indices[2] as usize]),
            (verts[indices[2] as usize], verts[indices[0] as usize]),
            (verts[indices[3] as usize], verts[indices[4] as usize]),
            (verts[indices[4] as usize], verts[indices[5] as usize]),
            (verts[indices[5] as usize], verts[indices[3] as usize]),
        ];

        // It's a valid dash if 2 condiitons are met:
        // 1. All vertices that make up the triangles are at a corner of the dash
        // 2. All 4 'edges' of the rectangular dash are represented in triangles
        for index in indices {
            let vert = verts[*index as usize];
            assert_eq!(
                vert[0].abs(),
                MIDLINE_X_MAG,
                "Expected dash vertex to have x magnitude {}, but got {}",
                MIDLINE_X_MAG,
                vert[0].abs()
            );
            assert!(
                (vert[1] == top_y) || (vert[1] == bot_y),
                "Expected dash vertex to have y of {} or {}, but got {}",
                top_y,
                bot_y,
                vert[1],
            );
            assert_eq!(
                vert[2], 0f32,
                "Expected dash vertex to have z value of 0, but got {}",
                vert[2],
            );
        }
        let mut edge_map: u8 = 0b0000; /* 4 bit mask of 4 edges being found */
        for edge in edges {
            if edge.0[0] != edge.1[0] {
                if (edge.0[1] == top_y) && (edge.1[1] == top_y) {
                    edge_map |= 0b0001; // Top Edge
                } else if (edge.0[1] == bot_y) && (edge.1[1] == bot_y) {
                    edge_map |= 0b0010; // Bottom Edge
                }
            } else if edge.0[1] != edge.1[1] {
                if (edge.0[0] < 0f32) && (edge.1[0] < 0f32) {
                    edge_map |= 0b0100; // Left Edge
                } else if (edge.0[0] > 0f32) && (edge.1[0] > 0f32) {
                    edge_map |= 0b1000; // Right Edge
                }
            }
        }
        assert!(
            edge_map == 0b1111,
            "Expected to find all 4 edges of dash, but at least one is missing. Bitmap {:b}",
            edge_map,
        );
    }
}
