use bevy::prelude::*;

fn main() {
	App::new()
		.add_plugins((
			DefaultPlugins,
			killcam_app::KillcamPlugin,
			net_app::NetPlugin,
			// Included even in client builds for P2P games
			server_app::ServerPlugin,
		))
		.run();
}
