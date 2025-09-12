//!
//! Contains code to setup and manage the paddles on either side of the pong screen,
//! and allow other code to query for paddle positional data for use in collision logic.
//!

// -------------------------------------------------------------------------------------------------
// Included Symbols

use bevy::ecs::query::QueryData;
use bevy::prelude::*;
use bevy::sprite::Anchor;

use crate::common::*;

// -------------------------------------------------------------------------------------------------
// Constants

pub const PADDLE_HEIGHT_AS_SCREEN_PCT: f32 = 0.15;
pub const PADDLE_ASPECT_RATIO: f32 = 0.15;
pub const PADDLE_MOVE_SPEED: f32 = ARENA_HEIGHT * 1.5;
pub const PADDLE_HEIGHT: f32 = PADDLE_HEIGHT_AS_SCREEN_PCT * ARENA_HEIGHT;
pub const PADDLE_WIDTH: f32 = PADDLE_HEIGHT * PADDLE_ASPECT_RATIO;
pub const PADDLE_CLAMP_Y: f32 = (ARENA_HEIGHT / 2f32) - (PADDLE_HEIGHT / 2f32);

// -------------------------------------------------------------------------------------------------
// Public API

///
/// The PaddlePlugin adds 2 paddles to the screen, one on each side.
/// It also handles user input to move the paddles up and down using W/S and ^/v keys.
/// There is also a read-only API exposed to query positional data about the paddles
/// for use in collision computation.
///
pub struct PaddlePlugin;

impl Plugin for PaddlePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_paddles.in_set(Systems::Startup))
            .add_systems(
                Update,
                handle_input_move_paddles.in_set(Systems::HandleInput),
            );
    }
}

/// These SystemSets are used to control any system ordering dependencies on this plugin
#[derive(SystemSet, Debug, Clone, Hash, PartialEq, Eq)]
pub enum Systems {
    /// Implements all logic to create the paddle entities. Must be in Startup.
    Startup,

    ///
    /// Implements all logic to retrieve user input events and update
    /// the paddle positions accordingly. Must be in Update.
    ///
    HandleInput,
}

///
/// Read-only marker component which is present on paddle entities.
/// Intended for use by other code modules to help avoid query component conflicts,
/// by using Without<PaddleMarker> in Query filters as needed.
///
#[derive(Component)]
pub struct PaddleMarker(PlayerId);

///
/// A custom QueryData which allows read-only access to the hitbox API.
/// The entrypoint for the API is a system with parameter Query<AllPaddleHitboxes>.
/// From there, the API allows an individual player hitbox to be selected,
/// and relevant data for the hitbox can be retrieved via the API.
///
#[derive(QueryData)]
pub struct AllPaddleHitboxes(&'static PaddleMarker, &'static Transform);

///
/// A type alias to allow more succinct access to the individual hitbox "items"
/// within AllPaddleHitboxes. The type itself represents a single paddle hitbox
/// within the world. It allows retrieval of several relevant hitbox-related values
/// of the paddle to be used in collision detection.
///
pub type PaddleHitbox<'w> = AllPaddleHitboxesItem<'w>;

impl<'w> PaddleHitbox<'w> {
    ///
    /// Given the query for all paddle hitboxes, retrieve the one specific to a
    /// particular PlayerId.
    ///
    pub fn from_query(query: Query<'w, '_, AllPaddleHitboxes>, player: PlayerId) -> Self {
        for item in query {
            if item.0.0 == player {
                return item as PaddleHitbox;
            }
        }
        panic!("PlayerId {player:?} was not found in AllPaddleHitboxes query.");
    }

    ///
    /// Get an origin point for the collision plane of this paddle. The plane
    /// is on the vertical face of the paddle nearest the center line of the arena.
    ///
    pub fn plane_origin(&self) -> Vec2 {
        let x_offset = match self.0.0 {
            Player1 => self.1.scale.x,
            Player2 => -self.1.scale.x,
        };

        self.1.translation.xy() + Vec2::new(x_offset, 0f32)
    }

    /// Get the topmost Y coordinate of the collision surface of the paddle.
    pub fn top_y(&self) -> f32 {
        self.1.translation.y + (self.1.scale.y / 2f32)
    }

    /// Get the bottommost Y coordinate of the collision surface of the paddle.
    pub fn bot_y(&self) -> f32 {
        self.1.translation.y - (self.1.scale.y / 2f32)
    }
}

// -------------------------------------------------------------------------------------------------
// Private Systems

//
// Creates two paddles - one for each player. One paddle is against the left edge of
// the screen, one is against the right edge. They are vertically centered to start.
//
fn setup_paddles(mut commands: Commands) {
    let paddle_size = Vec3::new(PADDLE_WIDTH, PADDLE_HEIGHT, 0f32);

    commands.spawn((
        PaddleMarker(Player1),
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
                z: Z_FOREGROUND,
            },
            scale: paddle_size,
            ..default()
        },
    ));

    commands.spawn((
        PaddleMarker(Player2),
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
                z: Z_FOREGROUND,
            },
            scale: paddle_size,
            ..default()
        },
    ));
}

// Checks relevant user inputs and updates positions of paddles accordingly.
fn handle_input_move_paddles(
    paddles: Query<(&mut Transform, &PaddleMarker)>,
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
) {
    let distance = time.delta_secs() * PADDLE_MOVE_SPEED;
    let (p1_trans, p2_trans) = paddles
        .into_iter()
        .map(|(t, pad)| (pad.0, &mut t.into_inner().translation))
        .as_per_player();

    match (keys.pressed(KeyCode::KeyW), keys.pressed(KeyCode::KeyS)) {
        (true, false) => {
            p1_trans.y = (p1_trans.y + distance).min(PADDLE_CLAMP_Y);
        }
        (false, true) => {
            p1_trans.y = (p1_trans.y - distance).max(-PADDLE_CLAMP_Y);
        }
        _ => (), // No p1 movement if neither or both are pressed
    }

    match (
        keys.pressed(KeyCode::ArrowUp),
        keys.pressed(KeyCode::ArrowDown),
    ) {
        (true, false) => {
            p2_trans.y = (p2_trans.y + distance).min(PADDLE_CLAMP_Y);
        }
        (false, true) => {
            p2_trans.y = (p2_trans.y - distance).max(-PADDLE_CLAMP_Y);
        }
        _ => (), // No p2 movement if neither or both are pressed
    }
}

// -------------------------------------------------------------------------------------------------
// Unit Tests

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::schedule::ScheduleBuildError;

    #[test]
    fn test_sys_add_setup() {
        let mut app = App::new();
        app.add_plugins(PaddlePlugin);

        // This ordering will lead to an error (which we expect) if the system
        // exists and is in the system set as it should be.
        app.configure_sets(Startup, Systems::Startup.before(setup_paddles));
        let init_result = app
            .world_mut()
            .try_schedule_scope(Startup, |world, sched| sched.initialize(world))
            .expect("Expected Startup schedule to exist in app");
        let Err(ScheduleBuildError::SetsHaveOrderButIntersect(..)) = init_result else {
            panic!(concat!(
                "Expected Startup schedule build to fail, ",
                "since 'setup_paddles' should be in Startup system set. But it succeeded"
            ));
        };
    }

    #[test]
    fn test_sys_add_handle_input() {
        let mut app = App::new();
        app.add_plugins(PaddlePlugin);

        // This ordering will lead to an error (which we expect) if the system
        // exists and is in the system set as it should be.
        app.configure_sets(Update, Systems::HandleInput.before(handle_input_move_paddles));
        let init_result = app
            .world_mut()
            .try_schedule_scope(Update, |world, sched| sched.initialize(world))
            .expect("Expected Update schedule to exist in app");
        let Err(ScheduleBuildError::SetsHaveOrderButIntersect(..)) = init_result else {
            panic!(concat!(
                "Expected Update schedule build to fail, ",
                "since 'handle_input_move_paddles' should be in Startup system set. ",
                "But it succeeded",
            ));
        };
    }

    #[test]
    fn test_setup_paddles_system() {
        let mut world = World::default();

        // Run the system and let it create entities we expect
        let setup_sys = world.register_system(setup_paddles);
        world.run_system(setup_sys).unwrap();

        // Show Without<PaddleMarker> works to guarantee disjoint queries
        let mut query = world.query_filtered::<Entity, Without<PaddleMarker>>();
        assert_eq!(
            query.iter(&world).count(),
            0,
            "Expected no items in query when using filter Without<PaddleMarker>"
        );

        // Validate paddles are created with sensible values
        let mut query_state = world.query::<AllPaddleHitboxes>();
        let p1_paddle = PaddleHitbox::from_query(query_state.query(&world), Player1);
        let p2_paddle = PaddleHitbox::from_query(query_state.query(&world), Player2);

    }

    #[test]
    fn test_handle_input_system() {
        todo!()
    }
}
