// Title for the app window
pub const PONG_WINDOW_TITLE: &str = "Pong Remake";

// Size of Pong "Arena" in world, which we want to keep focused on display
pub const ARENA_WIDTH: f32 = 16.0;
pub const ARENA_HEIGHT: f32 = 9.0;

// Sizing constraints for the window itself
pub const MIN_WINDOW_WIDTH: f32 = 160.0;
pub const MIN_WINDOW_HEIGHT: f32 = 90.0;
pub const MAX_WINDOW_WIDTH: f32 = 7680.0;
pub const MAX_WINDOW_HEIGHT: f32 = 4320.0;

// Initial window size (logical pixels)
pub const INITIAL_WINDOW_WIDTH: f32 = 1600.0;
pub const INITIAL_WINDOW_HEIGHT: f32 = 900.0;

// Sizing parameters for midline dashes
pub const MIDLINE_DASH_WIDTH: f32 = 0.005 * ARENA_WIDTH;
pub const MIDLINE_DASH_HEIGHT: f32 = 0.055 * ARENA_HEIGHT;

// Sizing parameters for paddles
pub const PADDLE_HEIGHT_AS_SCREEN_PCT: f32 = 0.15;
pub const PADDLE_ASPECT_RATIO: f32 = 0.15;

// Sizing parameters for ball
pub const BALL_SIZE_AS_SCREEN_HEIGHT_PCT: f32 = 0.02;

// Speed parameters for ball
pub const BALL_MOVE_SPEED: f32 = 0.9 * ARENA_WIDTH;

// Z-index value for secondary on-screen elements (to be behind balls/paddles)
pub const Z_BEHIND_GAMEPLAY: f32 = -1f32;

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum PlayerId {
    Player1,
    Player2,
}

pub use PlayerId::Player1;
pub use PlayerId::Player2;

pub trait AsPerPlayerData<T> {
    fn as_per_player(self) -> (T, T);
}

impl<T, U> AsPerPlayerData<T> for U
where
    U: Iterator<Item = (PlayerId, T)>,
{
    fn as_per_player(mut self) -> (T, T) {
        let item1 = self.next();
        let item2 = self.next();
        assert!(
            self.next().is_none(),
            "Expected 1 iterator entry for each player. Got more than 2."
        );

        match (item1, item2) {
            (Some(item1), Some(item2)) => {
                if item1.0 == Player1 {
                    assert!(
                        item2.0 == Player2,
                        "Expected 1 iterator entry for each player. Got 2 for Player 1"
                    );
                    (item1.1, item2.1)
                } else {
                    assert!(
                        item2.0 == Player1,
                        "Expected 1 iterator entry for each player. Got 2 for Player 2"
                    );
                    (item2.1, item1.1)
                }
            }
            _ => panic!("Expected 1 iterator entry for each player. Got less than 2."),
        }
    }
}
