use bevy::app::{AppLabel, MainSchedulePlugin, ScheduleRunnerPlugin};
use bevy::ecs::event::EventRegistry;
use bevy::ecs::schedule::{ExecutorKind, ScheduleLabel};
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy::state::app::StatesPlugin;
use sim::SimPlugin;
use sim::players::PlayerId;
use std::collections::VecDeque;

/// Label for the server SubApp.
#[derive(AppLabel, Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct ServerSim;

/// Sets up the server app which manages the source-of-truth game state and syncs it to clients.
///
/// The server app receives input actions from the network apps for all players, and simulates the
/// world forward. It then sends the resulting state to all clients for them to synchronize to.
///
/// The server app may run on the client machines for P2P games, or on a dedicated server for
/// better cheat resistance.
pub struct ServerPlugin;

impl Plugin for ServerPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(
			FixedUpdate,
			tick_server_sim.run_if(in_state(ServerState::Running)),
		);

		let mut srv_app = SubApp::new();

		// AppTypeRegistry is initialized in `App::default`. We want to share it with sub-apps.
		let reg = app.world().resource::<AppTypeRegistry>().clone();
		srv_app.insert_resource(reg);
		// Sub-apps have their own events. Shared events must be manually synchronized.
		srv_app.init_resource::<EventRegistry>();

		// sub-schedules run their systems in parallel, which is the default for `Schedule::new`,
		// so we can let them be automatically added with `add_systems`

		srv_app
			.add_plugins((SimPlugin,))
			.init_state::<ServerState>();

		app.insert_sub_app(ServerSim, srv_app);
	}

	fn cleanup(&self, app: &mut App) {
		let server_world = std::mem::take(app.sub_app_mut(ServerSim).world_mut());
		app.insert_resource(ServerWorld(server_world));
	}
}

/// Resource that holds the server world, taken from the `ServerApp` sub-app on
/// `ServerPlugin::cleanup`. Since the server simulation does not run automatically, but instead
/// needs to be run by the main app, it is taken from the sub-app and stored in the main app's world.
#[derive(Resource, Deref, DerefMut)]
pub struct ServerWorld(pub World);

impl ServerWorld {
	pub fn tick_simulation(&mut self) {
		self.run_schedule(Main);
	}
}

pub fn tick_server_sim(
	mut server_world: ResMut<ServerWorld>,
	player_inputs: Res<PlayerInputHistory>,
) {
	server_world.tick_simulation();
}

#[derive(States, Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum ServerState {
	#[default]
	NoActiveGame,
	Running,
	Finalizing,
}

#[derive(Resource, Clone, Debug)]
pub struct PlayerInputHistory {
	pub last: HashMap<PlayerId, PlayerInput>,
	pub inputs: VecDeque<HashMap<PlayerId, Option<PlayerInput>>>,
}

#[derive(Clone, Debug)]
pub struct PlayerInput {
	// todo
}
