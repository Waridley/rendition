use avian3d::PhysicsPlugins;
use bevy::ecs::schedule::{ExecutorKind, ScheduleLabel};
use bevy::prelude::*;
use std::time::Duration;

use crate::players::PlayerId;
use crate::spectators::SpectatorId;
pub use avian3d as phys;
use avian3d::prelude::{Gravity, Physics, PhysicsTime};
use bevy::app::{MainSchedulePlugin, ScheduleRunnerPlugin};
use bevy::diagnostic::FrameCountPlugin;
use bevy::time::{TimePlugin, TimeUpdateStrategy};

pub mod players;
pub mod spectators;

pub mod prelude {
	pub use super::SimPlugin;
	pub use avian3d::prelude::*;
}

pub const DT: Duration = Duration::from_micros(15625);

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
		let mut phys_t = Time::<Physics>::default();
		// Physics time is manually advanced alongside simulation time
		phys_t.pause();
		app
			// We could make a custom schedule, but just forcing the main schedule to run at a fixed rate
			// integrates better with other Bevy plugins.
			.insert_resource(TimeUpdateStrategy::ManualDuration(DT))
			.insert_resource(phys_t)
			.init_resource::<Time<Sim>>()
			.add_plugins((MinimalPlugins, PhysicsPlugins::default()))
			.register_type::<ClientId>()
			.insert_resource(Gravity(Vec3::NEG_Z * 9.81));
	}
}

/// The clock representing simulation time. Is set as the default `Time` during the simulation schedule.
#[derive(Debug, Reflect, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[reflect(Debug, PartialEq, Hash, Default)]
pub struct Sim {
	/// The fixed timestep to advance the simulation by.
	pub dt: Duration,
}

impl Default for Sim {
	fn default() -> Self {
		Self { dt: DT }
	}
}

/// Unique ID for each client in a match. May be a player or spectator.
#[derive(Component, Copy, Clone, Debug, PartialEq, Eq, Hash, Reflect, PartialOrd, Ord)]
pub enum ClientId {
	Player(PlayerId),
	Spectator(SpectatorId),
}
