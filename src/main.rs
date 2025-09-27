use bevy::prelude::*;
use bevy_transform_interpolation::interpolation::TransformInterpolationPlugin;

pub mod extract;
#[cfg(feature = "gui")]
pub mod gui;
#[cfg(not(feature = "gui"))]
pub mod headless;
pub mod net;

fn main() {
	App::new()
		.add_plugins((
			#[cfg(not(feature = "gui"))]
			headless::HeadlessPlugin,
			#[cfg(feature = "gui")]
			gui::GuiPlugin,
			// Included even in client builds for P2P games
			server_app::ServerPlugin,
			TransformInterpolationPlugin::default(),
			extrapolation::ExtrapolationPlugin::default(),
		))
		.add_systems(OnEnter(MainState::Splash), show_splash)
		.run();
}

#[derive(States, Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum MainState {
	#[default]
	Splash,
	MainMenu,
	Loading,
	InGame,
}

fn show_splash(mut next_state: ResMut<NextState<MainState>>) {
	info!("Todo: splash screen; going straight to MainMenu for now");
	next_state.set(MainState::MainMenu);
}

mod extrapolation {
	//! This is copied from Avian3D because the types are not public.
	//!
	//! Maybe I'll make an issue, but this is admittedly quite a niche use case, needing to manually
	//! add the `TransformExtrapolation` and not `PhysicsInterpolationPlugin`, but still use Avian's
	//! types as the `LinVelSource` and `AngVelSource`. However, the main app will not run any physics
	//! simulations, but velocities will be extracted from the active sim world for visualization.

	use bevy::ecs::query::QueryData;
	use bevy::math::Vec3;
	use bevy::prelude::{Component, Deref, DerefMut};
	use bevy_transform_interpolation::VelocitySource;
	use bevy_transform_interpolation::extrapolation::TransformExtrapolationPlugin;
	use sim::phys::math::Vector;
	use sim::prelude::{AngularVelocity, LinearVelocity};

	pub type ExtrapolationPlugin = TransformExtrapolationPlugin<LinVelSource, AngVelSource>;

	#[derive(QueryData)]
	pub struct LinVelSource;

	impl VelocitySource for LinVelSource {
		type Previous = PreviousLinearVelocity;
		type Current = LinearVelocity;

		fn previous(previous: &Self::Previous) -> Vec3 {
			previous.0
		}

		fn current(current: &Self::Current) -> Vec3 {
			current.0
		}
	}

	#[derive(QueryData)]
	pub struct AngVelSource;

	#[allow(clippy::unnecessary_cast)]
	impl VelocitySource for AngVelSource {
		type Previous = PreviousAngularVelocity;
		type Current = AngularVelocity;

		fn previous(previous: &Self::Previous) -> Vec3 {
			previous.0.0
		}

		fn current(current: &Self::Current) -> Vec3 {
			current.0
		}
	}

	#[derive(Component, Default, Deref, DerefMut)]
	pub struct PreviousLinearVelocity(Vector);

	#[derive(Component, Default, Deref, DerefMut)]
	pub struct PreviousAngularVelocity(AngularVelocity);
}
