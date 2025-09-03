use bevy::{app::AppLabel, prelude::*};
use bevy::ecs::event::EventRegistry;
use sim::SimPlugin;
use crate::players::PlayerId;
use crate::spectators::SpectatorId;

/// Sets up the game client, which reads player input, predicts the simulation,
/// sends inputs to the `NetApp` to forward to the server, receives confirmed
/// state from the server, rolls back to that state, and re-runs the simulation
/// multiple times to re-predict the current state.
pub struct ClientPlugin;

impl Plugin for ClientPlugin {
	fn build(&self, app: &mut App) {
		let mut client_app = SubApp::new();
		
		// AppTypeRegistry is initialized in `App::default`. We want to share it with sub-apps.
		let reg = app.world().resource::<AppTypeRegistry>().clone();
		client_app.insert_resource(reg);
		// Sub-apps have their own events. Shared events must be manually synchronized.
		client_app.init_resource::<EventRegistry>();
		
		client_app.add_plugins((
			MinimalPlugins,
			SimPlugin,
		));
		
		app.insert_sub_app(ClientApp, client_app);
	}
}

/// Label for the client SubApp.
/// 
/// This sub-app is responsible for reading player input, predicting the simulation,
/// sending inputs to the `NetApp` to forward to the server, receiving confirmed
/// state from the server, rolling back to that state, and re-running the simulation
/// multiple times to re-predict the current state. The state is then extracted into
/// the main world for rendering, unless the killcam app is currently active, in which
/// case the killcam app's world is rendered instead.
#[derive(AppLabel, Default, Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ClientApp;

/// Unique ID for each client in a match. May be a player or spectator.
#[derive(Component, Copy, Clone, Debug, PartialEq, Eq, Hash, Reflect, PartialOrd, Ord)]
pub enum ClientId {
	Player(PlayerId),
	Spectator(SpectatorId),
}
