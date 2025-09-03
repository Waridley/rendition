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
