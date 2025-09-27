use bevy::app::{MainSchedulePlugin, ScheduleRunnerPlugin};
use bevy::ecs::event::EventRegistry;
use bevy::ecs::schedule::{ExecutorKind, ScheduleLabel};
use bevy::state::app::StatesPlugin;
use bevy::{app::AppLabel, prelude::*};
use sim::SimPlugin;

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

		client_app
			.add_plugins((MainSchedulePlugin, StatesPlugin, SimPlugin))
			.init_state::<ClientState>();

		app.insert_sub_app(ClientApp, client_app);
	}

	fn cleanup(&self, app: &mut App) {
		info!("inserting ClientWorld");
		let client_world = std::mem::take(app.sub_app_mut(ClientApp).world_mut());
		app.insert_resource(ClientWorld(client_world));
	}
}

/// Resource that holds the client world, taken from the `ClientApp` sub-app on
/// `ClientPlugin::cleanup`. Since the client simulation does not run automatically, but instead
/// needs to be run by the main app, it is taken from the sub-app and stored in the main app's world.
#[derive(Resource, Deref, DerefMut)]
pub struct ClientWorld(pub World);

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

#[derive(States, Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum ClientState {
	#[default]
	NoActiveGame,
}
