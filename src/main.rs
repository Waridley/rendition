use bevy::prelude::*;

pub mod client;
pub mod extract;
pub mod net;
pub mod players;
pub mod spectators;

fn main() {
	App::new()
		.add_plugins((
			DefaultPlugins,
			client::ClientPlugin,
			killcam_app::KillcamPlugin,
			// Included even in client builds for P2P games
			server_app::ServerPlugin,
		))
		.run();
}
