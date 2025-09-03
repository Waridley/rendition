use avian3d::PhysicsPlugins;
use bevy::ecs::schedule::{ExecutorKind, ScheduleLabel};
use bevy::prelude::*;
use std::time::Duration;

pub use avian3d as phys;
use avian3d::prelude::{Gravity, Physics, PhysicsTime};

pub mod prelude {
	pub use super::SimPlugin;
	pub use avian3d::prelude::*;
}

/// Schedule that runs the game simulation.
///
/// **WARNING:** Do not modify this schedule outside of the `SimPlugin` unless you have a very good
/// reason. All apps need to be running the exact same simulation to ensure correctness and
/// deterministic rollback, prediction, and replay. Data should be synchronized in the World before
/// and after running this schedule, letting this schedule handle ticking the simulation forward.
///
/// May be run normally (once-per-frame) on the client,
/// on old data on the client for killcams, or on the server or client multiple times per frame to
/// catch back up to realtime after rolling back to a previous tick with new input sequences or
/// confirmed state from the server.
#[derive(ScheduleLabel, Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct SimMain;

/// Schedule that runs first in [`SimMain`]
///
/// **WARNING:** Do not modify this schedule outside of the `SimPlugin` unless you have a very good
/// reason. See [`SimMain`] for more information.
#[derive(ScheduleLabel, Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct SimFirst;

/// Schedule that runs before the physics schedule in [`SimMain`]
///
/// **WARNING:** Do not modify this schedule outside of the `SimPlugin` unless you have a very good
/// reason. See [`SimMain`] for more information.
#[derive(ScheduleLabel, Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct SimPrePhysics;

/// Schedule for avian3d to run in within [`SimMain`]
///
/// **WARNING:** Do not modify this schedule outside of the `SimPlugin` unless you have a very good
/// reason. See [`SimMain`] for more information.
#[derive(ScheduleLabel, Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct SimPhysics;

/// Schedule that runs after the physics schedule in [`SimMain`]
///
/// **WARNING:** Do not modify this schedule outside of the `SimPlugin` unless you have a very good
/// reason. See [`SimMain`] for more information.
#[derive(ScheduleLabel, Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct SimPostPhysics;

/// Schedule that runs last in [`SimMain`]
///
/// **WARNING:** Do not modify this schedule outside of the `SimPlugin` unless you have a very good
/// reason. See [`SimMain`] for more information.
#[derive(ScheduleLabel, Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct SimLast;

impl SimMain {
	/// Exclusive system that runs sub-schedules within the simulation schedule sequentially,
	/// like Bevy's built-in [`Main` schedule](bevy::app::main_schedule::Main).
	pub fn run(world: &mut World) {
		let span = trace_span!("SimMain::run");
		let _enter = span.enter();

		// Clone the old time so we can restore it after running the simulation
		let old_time = world.resource::<Time>().as_generic();

		// Advance simulation time and get dt to advance physics time by.
		let dt = world.resource_scope(|world, mut time: Mut<Time<Sim>>| {
			let dt = time.context().dt;
			time.advance_by(dt);
			// Set the default time to `Time<Sim>` so all systems use it by default
			*world.resource_mut::<Time>() = time.as_generic();
			dt
		});

		// Advance physics time by the same amount, since Avian3D uses `Time<Physics>`
		world.resource_mut::<Time<Physics>>().advance_by(dt);

		{
			let first = trace_span!("SimFirst");
			let _enter = first.enter();
			world.run_schedule(SimFirst);
		}
		{
			let pre_physics = trace_span!("SimPrePhysics");
			let _enter = pre_physics.enter();
			world.run_schedule(SimPrePhysics);
		}
		{
			let physics = trace_span!("SimPhysics");
			let _enter = physics.enter();
			world.run_schedule(SimPhysics);
		}
		{
			let post_physics = trace_span!("SimPostPhysics");
			let _enter = post_physics.enter();
			world.run_schedule(SimPostPhysics);
		}
		{
			let last = trace_span!("SimLast");
			let _enter = last.enter();
			world.run_schedule(SimLast);
		}

		// Restore the old default time so other systems don't use `Time<Sim>` by accident.
		*world.resource_mut::<Time>() = old_time;
	}
}

/// Sets up the simulation schedule. This must be added to all apps that need to simulate the game.
///
/// This is the sole source of truth for the game simulation. All other apps must run the same
/// simulation, deterministically, to ensure correctness and consistency across rollback,
/// prediction, and replay.
///
/// Data should be synchronized in the World via other plugins before and after running
/// `SimSchedule`, and this plugin should be used to modify the simulation itself. Player actions
/// should be stored in a queue that associates them with each tick, and any confirmed state from
/// the server that should override prediction should be applied in-between ticks.
pub struct SimPlugin;

impl Plugin for SimPlugin {
	fn build(&self, app: &mut App) {
		let mut sim_schedule = Schedule::new(SimMain);
		sim_schedule.set_executor_kind(ExecutorKind::SingleThreaded);
		let mut phys_t = Time::<Physics>::default();
		phys_t.pause();
		app.add_schedule(Schedule::new(SimMain))
			.add_schedule(Schedule::new(SimFirst))
			.add_schedule(Schedule::new(SimPrePhysics))
			.add_schedule(Schedule::new(SimPhysics))
			.add_schedule(Schedule::new(SimPostPhysics))
			.add_schedule(Schedule::new(SimLast))
			.add_systems(SimMain, SimMain::run)
			.add_plugins(PhysicsPlugins::new(SimPhysics))
			.insert_resource(Gravity(Vec3::NEG_Z * 9.81))
			.insert_resource(Time::<Sim>::default())
			.insert_resource(phys_t);
	}
}

/// The clock representing simulation time. Is set as the default `Time` during the simulation schedule.
pub struct Sim {
	/// The fixed timestep to advance the simulation by.
	dt: Duration,
}

impl Default for Sim {
	fn default() -> Self {
		Self {
			dt: Duration::from_secs_f64(1.0 / 64.0),
		}
	}
}
