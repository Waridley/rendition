use bevy::app::AppLabel;
use bevy::ecs::event::EventRegistry;
use bevy::prelude::*;

use sim::SimPlugin;

/// Label for the killcam SubApp.
#[derive(AppLabel, Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct KillcamApp;

/// Sets up the killcam system.
///
/// The killcam world keeps a snapshot of an earlier state of the confirmed world from the server,
/// and the sequence of inputs needed to replay the world up to the current confirmed state.
/// When killcams play, the client and killcam worlds are swapped. The killcam world does not accept
/// game input, only necessary actions like "skip", "pause", etc. When the killcam world is active,
/// the client world is still running in the background, but is not rendered.
pub struct KillcamPlugin;

impl Plugin for KillcamPlugin {
	fn build(&self, app: &mut App) {
		let mut kc_app = SubApp::new();

		// AppTypeRegistry is initialized in `App::default`. We want to share it with sub-apps.
		let reg = app.world().resource::<AppTypeRegistry>().clone();
		kc_app.insert_resource(reg);
		// Sub-apps have their own events. Shared events must be manually synchronized.
		kc_app.init_resource::<EventRegistry>();

		kc_app.add_plugins((MinimalPlugins, SimPlugin));

		app.insert_sub_app(KillcamApp, kc_app);
	}
}
