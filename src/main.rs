use bevy::prelude::*;

mod client;
mod extract;
mod net;
mod players;
mod spectators;

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
