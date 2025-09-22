//!
//! This module contains a subset of items that are relevant across the pong codebase
//! and will be included by many of the core modules.
//!

// -------------------------------------------------------------------------------------------------
// Constants

/// Width of pong Arena in world units
pub const ARENA_WIDTH: f32 = 16.0;
/// Height of pong Arena in world units
pub const ARENA_HEIGHT: f32 = 9.0;

/// Z index for background
pub const Z_BACKGROUND: f32 = -2f32;
/// Z index for components overlayed on background but behind core gameplay
pub const Z_BEHIND_GAMEPLAY: f32 = -1f32;
/// Z index for components in the foreground, in front of core gameplay
pub const Z_FOREGROUND: f32 = 1f32;

// -------------------------------------------------------------------------------------------------
// Re-Exports

pub use PlayerId::Player1;
pub use PlayerId::Player2;

// -------------------------------------------------------------------------------------------------
// Public Types

/// PlayerId to differentiate between players 1 and 2 throughout game logic
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum PlayerId {
    Player1,
    Player2,
}

// -------------------------------------------------------------------------------------------------
// Public Traits and Blanket Impls

///
/// A trait that can be implemented for any type that contains some data T for both
/// of the 2 players in the game. It allows retrieval of the data per-player.
///
pub trait AsPerPlayerData<T> {
    ///
    /// Required Method:
    /// Consumes the value and returns a tuple of T types for players 1 and 2.
    /// The first tuple item is for player 1. The second is for player 2.
    ///
    fn as_per_player(self) -> (T, T);
}

impl<T, U> AsPerPlayerData<T> for U
where
    U: Iterator<Item = (PlayerId, T)>,
{
    ///
    /// Consumes the iterator (assuming it contains exactly 1 entry for each player)
    /// and identifies which player each item T belongs to. Then returns the appropriate tuple.
    ///
    /// This is intended for common use in query results that exist for each player. The
    /// query iterator can be mapped to the appropriate iterator type (PlayerId, T) and
    /// then this function can be called to return the p1 and p2 data as a tuple.
    ///
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
