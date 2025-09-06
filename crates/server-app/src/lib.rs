use bevy::app::AppLabel;
use bevy::ecs::event::EventRegistry;
use bevy::ecs::schedule::{ExecutorKind, ScheduleLabel};
use bevy::prelude::*;
use sim::{SimMain, SimPlugin};

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
		let mut srv_app = SubApp::new();

		// AppTypeRegistry is initialized in `App::default`. We want to share it with sub-apps.
		let reg = app.world().resource::<AppTypeRegistry>().clone();
		srv_app.insert_resource(reg);
		// Sub-apps have their own events. Shared events must be manually synchronized.
		srv_app.init_resource::<EventRegistry>();

		let mut sched = Schedule::new(ServerSchedule);
		// ServerSchedule runs sub-schedules sequentially
		sched.set_executor_kind(ExecutorKind::SingleThreaded);
		
		// sub-schedules run their systems in parallel, which is the default for `Schedule::new`,
		// so we can let them be automatically added with `add_systems`
		
		srv_app
			.add_plugins((MinimalPlugins, SimPlugin))
			.add_schedule(sched)
			.add_systems(ServerSchedule, ServerSchedule::run)
			.init_state::<ServerState>();

		app.insert_sub_app(ServerApp, srv_app);
	}
	
	fn cleanup(&self, app: &mut App) {
		let server_world = std::mem::take(app.sub_app_mut(ServerApp).world_mut());
		app.insert_resource(ServerWorld(server_world));
	}
}

/// Resource that holds the server world, taken from the `ServerApp` sub-app on
/// `ServerPlugin::cleanup`. Since the server simulation does not run automatically, but instead
/// needs to be run by the main app, it is taken from the sub-app and stored in the main app's world.
#[derive(Resource, Deref, DerefMut)]
pub struct ServerWorld(pub World);

/// Schedule for the server app. Only runs on the host during P2P games or in dedicated servers.
#[derive(ScheduleLabel, Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct ServerSchedule;

impl ServerSchedule {
	pub fn run(world: &mut World) {
		let span = trace_span!("ServerSchedule::run");
		let pre_sim = trace_span!("ServerPreSim");
		let sim = trace_span!("SimMain");
		let post_sim = trace_span!("ServerPostSim");
		
		let _enter = span.enter();
		{
			let _enter = pre_sim.enter();
			let _ = world.try_run_schedule(ServerPreSim);
		}
		{
			let _enter = sim.enter();
			let _ = world.try_run_schedule(SimMain);
		}
		{
			let _enter = post_sim.enter();
			let _ = world.try_run_schedule(ServerPostSim);
		}
	}
}

/// Schedule that runs before the simulation schedule on the server.
#[derive(ScheduleLabel, Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct ServerPreSim;

/// Schedule that runs after the simulation schedule on the server.
#[derive(ScheduleLabel, Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct ServerPostSim;

#[derive(States, Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum ServerState {
	#[default]
	NoActiveGame,
}
