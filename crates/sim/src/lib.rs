use std::time::Duration;
use bevy::ecs::schedule::{ExecutorKind, ScheduleLabel};
use bevy::prelude::*;
use avian3d::PhysicsPlugins;

pub use avian3d as phys;
use avian3d::prelude::{Gravity, Physics, PhysicsTime};
use bevy::log::Level;
use bevy::log::tracing::span;

pub mod prelude {
	pub use avian3d::prelude::*;
	pub use super::SimPlugin;
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
pub struct SimSchedule;

impl SimSchedule {
	pub fn run_sub_schedules(world: &mut World) {
		span!(Level::TRACE, "SimSchedule::run_sub_schedules");
		let old_time = world.resource::<Time>().as_generic();
		let dt = world.resource_scope(|world, mut time: Mut<Time<Sim>>| {
			*world.resource_mut::<Time>() = time.as_generic();
			let dt = time.context().dt;
			time.advance_by(dt);
			dt
		});
		world.resource_mut::<Time<Physics>>()
			.advance_by(dt);
		world.run_schedule(SimPhysicsSchedule);
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
		let mut sim_schedule = Schedule::new(SimSchedule);
		sim_schedule.set_executor_kind(ExecutorKind::SingleThreaded);
		let mut phys_t = Time::<Physics>::default();
		phys_t.pause();
		app
			.add_schedule(Schedule::new(SimSchedule))
			.add_systems(SimSchedule, SimSchedule::run_sub_schedules)
			.add_plugins(PhysicsPlugins::new(SimPhysicsSchedule))
			.insert_resource(Gravity(Vec3::NEG_Z * 9.81))
			.insert_resource(Time::<Sim>::default())
			.insert_resource(phys_t);
	}
}

/// Schedule for avian3d to run in within the simulation schedule.
#[derive(ScheduleLabel, Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct SimPhysicsSchedule;

/// The clock representing simulation time.
pub struct Sim {
	dt: Duration,
}

impl Default for Sim {
	fn default() -> Self {
		Self {
			dt: Duration::from_secs_f64(1.0 / 64.0),
		}
	}
}
