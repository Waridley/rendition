use bevy::prelude::*;
use bevy::app::AppLabel;
use bevy::ecs::schedule::ScheduleLabel;
use sim::{SimPlugin, SimSchedule};

/// Label for the server SubApp.
#[derive(AppLabel, Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct ServerApp;

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
		let mut server_app = SubApp::new();
		
		server_app
			.add_plugins(SimPlugin)
			.add_schedule(Schedule::new(ServerSchedule))
			.init_resource::<ServerState>()
			.add_systems(ServerSchedule, ServerSchedule::run.run_if(server_running))
			.add_systems(Update, hello_world);
		
		app
			.insert_sub_app(ServerApp, server_app);
	}
}

/// Schedule for the server app. Only runs during P2P games or in dedicated servers.
#[derive(ScheduleLabel, Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct ServerSchedule;

impl ServerSchedule {
	pub fn run(world: &mut World) {
		world.run_schedule(SimSchedule);
	}
}

/// Run condition for the server schedule.
fn server_running(state: Res<ServerState>) -> bool {
	state.running
}

#[derive(Resource, Default, Debug, Reflect)]
#[reflect(Resource)]
pub struct ServerState {
	/// Whether the server is currently running. Defaults to false. Should be set when a game starts
	/// *and* the app is running on either a dedicated server or the host client in a P2P game.
	pub running: bool,
}

fn hello_world() {
	info!("Hello World!");
}
