use bevy::ecs::event::EventRegistry;
use bevy::ecs::schedule::{ExecutorKind, ScheduleLabel};
use bevy::{app::AppLabel, prelude::*};
use sim::{SimMain, SimPlugin};

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
		
		let mut sched = Schedule::new(ClientSchedule);
		// ClientSchedule runs sub-schedules sequentially
		sched.set_executor_kind(ExecutorKind::SingleThreaded);
		
		client_app
			.add_plugins((MinimalPlugins, SimPlugin))
			.add_schedule(sched)
			// sub-schedules run in parallel (default for `Schedule::new`)
			.add_schedule(Schedule::new(ClientPreSim))
			.add_schedule(Schedule::new(ClientPostSim))
			.add_systems(ClientSchedule, ClientSchedule::run);
		
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

/// Schedule for the client app. Only runs on clients.
///
/// Runs [`ClientPreSim`], [`SimMain`], then [`ClientPostSim`].
#[derive(ScheduleLabel, Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct ClientSchedule;

impl ClientSchedule {
	pub fn run(world: &mut World) {
		let span = trace_span!("ClientSchedule::run");
		let _enter = span.enter();
		
		{
			let pre_sim = trace_span!("ClientPreSim");
			let _enter = pre_sim.enter();
			world.run_schedule(ClientPreSim);
		}
		{
			let sim = trace_span!("SimMain");
			let _enter = sim.enter();
			world.run_schedule(SimMain);
		}
		{
			let post_sim = trace_span!("ClientPostSim");
			let _enter = post_sim.enter();
			world.run_schedule(ClientPostSim);
		}
	}
}

/// Schedule that runs before the simulation schedule on the client.
#[derive(ScheduleLabel, Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct ClientPreSim;

/// Schedule that runs after the simulation schedule on the client.
#[derive(ScheduleLabel, Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct ClientPostSim;
