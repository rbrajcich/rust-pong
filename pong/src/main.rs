use bevy::prelude::App;

use pong::PongPlugin;

fn main() {
    App::new().add_plugins(PongPlugin).run();
}
