use bevy::prelude::*;

#[cfg(not(feature = "gui"))]
pub mod headless;
pub mod extract;
pub mod net;
#[cfg(feature = "gui")]
pub mod gui;

fn main() {
	App::new()
		.add_plugins((
			#[cfg(not(feature = "gui"))] headless::HeadlessPlugin,
			#[cfg(feature = "gui")] gui::GuiPlugin,
			// Included even in client builds for P2P games
			server_app::ServerPlugin,
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
