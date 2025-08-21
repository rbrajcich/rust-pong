use bevy::prelude::App;

use pong::PongPlugins;

fn main() {
    App::new()
        .add_plugins(PongPlugins)
        .run();
}
