use bevy::prelude::*;
use bevy::app::AppLabel;

/// Label for the network-syncing SubApp.
#[derive(AppLabel, Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct NetApp;

/// Sets up the network-syncing app which facilitates communication between the client and server.
///
/// The network app is responsible for sending client input to the server and syncing server state
/// to clients. This means, primarily, the network app will receive input actions from the client,
/// and actual entity state from the server. The client might be able to "hint" to the server what
/// it *thinks* happened for various reasons, but the server is the final authority on state.
pub struct NetPlugin;

impl Plugin for NetPlugin {
	fn build(&self, app: &mut App) {
		let mut net_app = SubApp::new();
		
		net_app.add_systems(Update, hello_world);
		
		app
			.insert_sub_app(NetApp, net_app);
	}
}

fn hello_world() {
	info!("Hello World!");
}
