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

// Sizing parameters for text
pub const SCORE_FONT_SIZE_AS_SCREEN_PCT: f32 = 0.2;
pub const WIN_FONT_SIZE_AS_SCREEN_PCT: f32 = 0.04;

// Winning score
pub const WINNING_SCORE: u8 = 10;

pub enum PlayerId {
    Player1,
    Player2,
}

pub use PlayerId::Player1;
pub use PlayerId::Player2;
